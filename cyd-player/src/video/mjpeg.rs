use core::{
    cell::{Cell, RefCell},
    fmt,
    ops::Range,
};

use alloc::vec;
use memchr::memmem;

use crate::{error::Error, video::decoder::Decoder};
use cyd_encoder::format::{FormatHeader, mjpeg::MjpegHeader};
use embedded_graphics::{
    Drawable,
    geometry::Point,
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle as GraphicsRectangle,
};
use embedded_io::{Read, ReadExactError, Seek};
use tjpgdec_rs::{JpegDecoder, MINIMUM_POOL_SIZE, MemoryPool};
extern crate alloc;

pub struct MjpegDecoder<R>
where
    R: Read + Seek,
{
    header: MjpegHeader,
    reader: R,
    soi_finder: memmem::Finder<'static>,
    eoi_finder: memmem::Finder<'static>,
    decode_buffer_valid: Range<usize>,
}

// 15K buffer to read compressed JPG 320x240 image plus pool
pub const DECODE_SIZE: usize = (15 * 1024) + MINIMUM_POOL_SIZE;

mod markers {
    pub const SOI: &[u8; 2] = &[0xFF, 0xD8];
    pub const EOI: &[u8; 2] = &[0xFF, 0xD9];
}

impl<R: Read + Seek> MjpegDecoder<R> {
    fn find_jpeg(&self, buffer: &[u8]) -> Option<Range<usize>> {
        let soi_pos = self.soi_finder.find(buffer)?;
        let eoi_pos = self.eoi_finder.find(&buffer[soi_pos..])?;
        let eoi_absolute = soi_pos + eoi_pos + markers::EOI.len();
        Some(soi_pos..eoi_absolute)
    }
}

impl<R, D> Decoder<R, D, 1, MjpegHeader, { DECODE_SIZE }> for MjpegDecoder<R>
where
    R: Read + Seek,
    D: DrawTarget<Color = Rgb565>,
    D::Error: fmt::Debug,
{
    type DecoderError = tjpgdec_rs::Error;
    type ImageDrawable<'a> = JpegDrawable<'a>;

    fn new(mut reader: R) -> Result<Self, ReadExactError<R::Error>> {
        let mut buffer = [0u8; 1];
        reader.read_exact(&mut buffer)?;
        let header = MjpegHeader::parse(&buffer);

        Ok(Self {
            header,
            reader,
            soi_finder: memmem::Finder::new(markers::SOI),
            eoi_finder: memmem::Finder::new(markers::EOI),
            decode_buffer_valid: 0..0,
        })
    }

    fn header(&self) -> &MjpegHeader {
        &self.header
    }

    fn decode_into<'a>(
        &mut self,
        buffer: &'a mut [u8; DECODE_SIZE],
    ) -> Result<Self::ImageDrawable<'a>, Error<R::Error, Self::DecoderError, D::Error>> {
        let [pool_buffer, decode_buffer] = buffer
            .get_disjoint_mut([0..MINIMUM_POOL_SIZE, MINIMUM_POOL_SIZE..DECODE_SIZE])
            .unwrap();
        // Shift valid contents to beginning
        if self.decode_buffer_valid.start > 0 {
            decode_buffer.copy_within(self.decode_buffer_valid.clone(), 0);
            self.decode_buffer_valid = 0..self.decode_buffer_valid.len();
        }
        let decode_buffer_len = decode_buffer.len();
        // Read into remaining unused buffer
        let read_len = self
            .reader
            .read(&mut decode_buffer[self.decode_buffer_valid.end..decode_buffer_len])
            .map_err(Error::ReadError)?;
        self.decode_buffer_valid.end += read_len;
        if let Some(jpeg_range) = self.find_jpeg(&decode_buffer[self.decode_buffer_valid.clone()]) {
            let end = jpeg_range.end;
            let jpeg_data = &decode_buffer[jpeg_range];
            self.decode_buffer_valid = end..self.decode_buffer_valid.end;
            JpegDrawable::new(pool_buffer, jpeg_data)
        } else {
            Err(Error::VideoEof)
        }
    }

    fn render<'a>(
        &'a self,
        image: Image<Self::ImageDrawable<'a>>,
        display: &mut D,
    ) -> Result<(), Error<R::Error, Self::DecoderError, D::Error>> {
        image.draw(display).map_err(Error::DisplayError)?;
        Ok(())
    }
}

pub struct JpegDrawable<'a> {
    jpeg_data: &'a [u8],
    decoder: RefCell<JpegDecoder<'a>>,
}

impl<'a> JpegDrawable<'a> {
    fn new<E, D>(
        pool_buffer: &'a mut [u8],
        jpeg_data: &'a [u8],
    ) -> Result<Self, Error<E, tjpgdec_rs::Error, D>>
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
