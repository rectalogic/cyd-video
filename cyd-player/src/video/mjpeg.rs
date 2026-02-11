use core::{
    cell::{Cell, RefCell},
    convert::Infallible,
    fmt,
};

use crate::{error::Error, video::decoder::Decoder};
use alloc::vec;

use embedded_graphics::{
    Drawable,
    geometry::Point,
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle as GraphicsRectangle,
};
use tjpgdec_rs::{JpegDecoder, MINIMUM_POOL_SIZE, MemoryPool};
extern crate alloc;

#[derive(Default)]
pub struct MjpegDecoder {}

// 15K buffer to read compressed JPG 320x240 image plus pool
pub const MAX_ENCODED_SIZE: usize = (15 * 1024) + MINIMUM_POOL_SIZE;

impl MjpegDecoder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<D> Decoder<D> for MjpegDecoder
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: fmt::Debug,
{
    type ImageDrawable<'a> = JpegDrawable<'a>;

    fn decode_frame<'a>(
        &mut self,
        frame_buffer: &'a mut [u8],
        frame_size: usize,
    ) -> Result<Self::ImageDrawable<'a>, Error<Infallible, D::Error>> {
        // 8 byte alignment
        let pool_start = frame_size + frame_buffer[frame_size..].as_ptr().align_offset(8);
        let [jpeg_data, pool_buffer] = frame_buffer
            .get_disjoint_mut([0..frame_size, pool_start..frame_buffer.len()])
            .unwrap();
        JpegDrawable::new(pool_buffer, jpeg_data)
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

pub struct JpegDrawable<'a> {
    jpeg_data: &'a [u8],
    decoder: RefCell<JpegDecoder<'a>>,
}

impl<'a> JpegDrawable<'a> {
    fn new<E, D>(pool_buffer: &'a mut [u8], jpeg_data: &'a [u8]) -> Result<Self, Error<E, D>>
    where
        E: fmt::Debug,
        D: fmt::Debug,
    {
        let mut pool = MemoryPool::new(pool_buffer);
        let mut decoder = JpegDecoder::new();
        decoder
            .prepare(jpeg_data, &mut pool)
            .map_err(Error::DecodeErrors)?;
        Ok(Self {
            jpeg_data,
            decoder: RefCell::new(decoder),
        })
    }

    fn render<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let display_error: Cell<Option<D::Error>> = Cell::new(None);
        let mut decoder = self.decoder.borrow_mut();
        let mcu_size = decoder.mcu_buffer_size();
        let work_size = decoder.work_buffer_size();
        let mut mcu_buffer = vec![0i16; mcu_size];
        let mut work_buffer = vec![0u8; work_size];
        if let Err(e) = decoder.decompress(
            self.jpeg_data,
            0,
            &mut mcu_buffer,
            &mut work_buffer,
            &mut |_decoder, bitmap, jpeg_rect| {
                let target_rect = GraphicsRectangle::with_corners(
                    Point::new(jpeg_rect.left as i32, jpeg_rect.top as i32),
                    Point::new(jpeg_rect.right as i32, jpeg_rect.bottom as i32),
                );
                let pixels = bitmap
                    .chunks_exact(3)
                    .map(|pixel| Rgb565::new(pixel[0] >> 3, pixel[1] >> 2, pixel[2] >> 3));
                // We can't return custom errors from the output function
                // https://docs.rs/tjpgdec-rs/0.4.0/tjpgdec_rs/type.OutputCallback.html
                if let Err(e) = target.fill_contiguous(&target_rect, pixels) {
                    display_error.set(Some(e));
                    return Ok(false);
                }
                Ok(true)
            },
        ) {
            // Not sure how we can return an error here
            log::error!("jpeg decode error: {e:?}");
        }
        if let Some(e) = display_error.take() {
            return Err(e);
        }
        Ok(())
    }
}

impl ImageDrawable for JpegDrawable<'_> {
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        self.render(target)
    }

    fn draw_sub_image<D>(
        &self,
        target: &mut D,
        area: &GraphicsRectangle,
    ) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        self.draw(&mut target.translated(-area.top_left).clipped(area))
    }
}

impl OriginDimensions for JpegDrawable<'_> {
    fn size(&self) -> Size {
        let decoder = self.decoder.borrow();
        Size::new(decoder.width() as u32, decoder.height() as u32)
    }
}
