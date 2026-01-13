use crate::{error::Error, video::decoder::Decoder};
use cyd_encoder::format::{FormatHeader, yuv::YuvHeader};
use display_interface::DisplayError;
use embedded_graphics::{
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};
use embedded_io::{Read, ReadExactError, Seek, SeekFrom};

pub struct YuvDecoder<R>
where
    R: Read + Seek,
{
    header: YuvHeader,
    reader: R,
}

pub const DECODE_SIZE: usize = (YuvHeader::MAX_WIDTH * YuvHeader::MAX_HEIGHT)
    + (YuvHeader::MAX_WIDTH * YuvHeader::MAX_HEIGHT) / 2;

impl<R, D> Decoder<R, D, 5, YuvHeader, { DECODE_SIZE }> for YuvDecoder<R>
where
    R: Read + Seek,
    D: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    type DecoderError = R::Error;
    type ImageDrawable<'a>
        = Pixels<'a>
    where
        Self: 'a;

    fn new(mut reader: R) -> Result<Self, ReadExactError<R::Error>> {
        let mut buffer = [0u8; 5];
        reader.read_exact(&mut buffer)?;
        let header = YuvHeader::parse(&buffer);
        Ok(Self { header, reader })
    }

    fn header(&self) -> &YuvHeader {
        &self.header
    }

    fn decode_into<'a>(
        &mut self,
        buffer: &'a mut [u8; DECODE_SIZE],
    ) -> Result<Pixels<'a>, Error<R::Error, Self::DecoderError>> {
        let width = self.header.width() as u32;
        let height = self.header.height() as u32;
        let buffer = &mut buffer[..((width * height) + (width * height) / 2) as usize];
        let size = Size::new(width, height);
        match self.reader.read_exact(buffer) {
            Ok(_) => {}
            Err(ReadExactError::UnexpectedEof) => {
                self.reader
                    .seek(SeekFrom::Start(YuvHeader::header_size() as u64))
                    .map_err(Error::ReadError)?;
                return Err(Error::LoopEof);
            }
            Err(ReadExactError::Other(e)) => return Err(Error::ReadError(e)),
        }
        Ok(Pixels::new(buffer, size))
    }

    fn render<'a>(
        &'a self,
        image: Image<Self::ImageDrawable<'a>>,
        display: &mut D,
    ) -> Result<(), Error<R::Error, Self::DecoderError>> {
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
