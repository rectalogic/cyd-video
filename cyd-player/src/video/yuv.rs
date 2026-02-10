use crate::{display, error::Error, video::decoder::Decoder};
use core::{convert::Infallible, fmt};
use embedded_graphics::{
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};

pub const MAX_ENCODED_SIZE: usize = (display::DISPLAY_WIDTH * display::DISPLAY_HEIGHT) as usize
    + (display::DISPLAY_WIDTH * display::DISPLAY_HEIGHT) as usize / 2;

pub struct YuvDecoder {
    width: u32,
    height: u32,
}

impl YuvDecoder {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl<D> Decoder<D> for YuvDecoder
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: fmt::Debug,
{
    type ImageDrawable<'a> = Pixels<'a>;

    fn decode_frame<'a>(
        &mut self,
        frame_buffer: &'a mut [u8],
        frame_size: usize,
    ) -> Result<Self::ImageDrawable<'a>, Error<Infallible, D::Error>> {
        let size = Size::new(self.width, self.height);
        Ok(Pixels::new(&frame_buffer[..frame_size], size))
    }

    fn render<'a>(
        &'a self,
        image: Image<Self::ImageDrawable<'a>>,
        display: &mut D,
    ) -> Result<(), Error<Infallible, D::Error>> {
        image.draw(display).map_err(Error::DisplayError)?;
        Ok(())
    }
}

pub struct Pixels<'a> {
    yuv: &'a [u8],
    size: Size,
}

impl<'a> Pixels<'a> {
    fn new(yuv: &'a [u8], size: Size) -> Self {
        Self { yuv, size }
    }

    fn pixels(&'a self) -> impl Iterator<Item = Rgb565> + 'a {
        let width = self.size.width as usize;
        let height = self.size.height as usize;
        let y_plane_len = width * height;
        let uv_plane_len = (width / 2) * (height / 2);

        (0..height).flat_map(move |y| {
            (0..width).map(move |x| {
                let y_index = y * width + x;
                let y_val = self.yuv[y_index] as f32;

                let uv_x = x / 2;
                let uv_y = y / 2;
                let uv_index = uv_y * (width / 2) + uv_x;

                let u = self.yuv[y_plane_len + uv_index] as f32 - 128.0;
                let v = self.yuv[y_plane_len + uv_plane_len + uv_index] as f32 - 128.0;

                // --- BT.709 full-range ---
                let r = y_val + 1.5748 * v;
                let g = y_val - 0.1873 * u - 0.4681 * v;
                let b = y_val + 1.8556 * u;

                let r8 = r.clamp(0.0, 255.0) as u8;
                let g8 = g.clamp(0.0, 255.0) as u8;
                let b8 = b.clamp(0.0, 255.0) as u8;

                // Convert 8-bit values to RGB565 bit depths
                // R: 5 bits (0-31), G: 6 bits (0-63), B: 5 bits (0-31)
                Rgb565::new(r8 >> 3, g8 >> 2, b8 >> 3)
            })
        })
    }
}

impl ImageDrawable for Pixels<'_> {
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        target.fill_contiguous(&self.bounding_box(), self.pixels())
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.draw(&mut target.translated(-area.top_left).clipped(area))
    }
}

impl OriginDimensions for Pixels<'_> {
    fn size(&self) -> Size {
        self.size
    }
}
