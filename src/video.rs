use embedded_graphics::{
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};
use embedded_io::{Read, ReadExactError};
use esp_hal::{
    delay::Delay,
    time::{Duration, Instant},
};

const MAX_WIDTH: usize = 320;
const MAX_HEIGHT: usize = 240;
const CENTER: Point = Point::new((MAX_WIDTH / 2) as i32, (MAX_HEIGHT / 2) as i32);

#[derive(Debug)]
pub enum Error<R, D>
where
    R: Read,
    D: DrawTarget<Color = Rgb565>,
{
    ReadError(ReadExactError<R::Error>),
    DrawError(D::Error),
}

impl<R, D> From<ReadExactError<R::Error>> for Error<R, D>
where
    R: Read,
    D: DrawTarget<Color = Rgb565>,
{
    fn from(value: ReadExactError<R::Error>) -> Self {
        Error::ReadError(value)
    }
}

pub struct Video {
    // yuv420 320x240
    buffer: [u8; (MAX_WIDTH * MAX_HEIGHT) + (MAX_WIDTH * MAX_HEIGHT) / 2],
    width: u32,
    height: u32,
    fps: u8,
}

impl Video {
    pub fn new<R: Read>(mut reader: R) -> Result<Self, ReadExactError<R::Error>> {
        let (width, height, fps) = read_header(&mut reader)?;
        Ok(Self {
            buffer: [0u8; _],
            width: width as u32,
            height: height as u32,
            fps,
        })
    }

    pub fn play<R, D>(&mut self, mut reader: R, display: &mut D) -> Result<(), Error<R, D>>
    where
        R: Read,
        D: DrawTarget<Color = Rgb565>,
    {
        let delay = Delay::new();
        let frame_duration = Duration::from_micros((1000 * 1000) / self.fps as u64);
        let mut start: Option<Instant> = None;
        loop {
            let buffer = &mut self.buffer
                [..((self.width * self.height) + (self.width * self.height) / 2) as usize];
            reader.read_exact(buffer)?;
            let pixels = Pixels::new(buffer, Size::new(self.width, self.height));
            let image = Image::with_center(&pixels, CENTER);
            if let Some(start) = start {
                delay.delay(frame_duration - start.elapsed());
            }
            start = Some(Instant::now());
            image.draw(display).map_err(Error::DrawError)?;
        }
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

pub const HEADER_SIZE: usize = 5;

pub fn read_header<R: Read>(mut reader: R) -> Result<(u16, u16, u8), ReadExactError<R::Error>> {
    let mut buf = [0u8; HEADER_SIZE];
    reader.read_exact(&mut buf)?;

    let width = u16::from_le_bytes([buf[0], buf[1]]);
    let height = u16::from_le_bytes([buf[2], buf[3]]);
    let fps = buf[4];

    Ok((width, height, fps))
}
