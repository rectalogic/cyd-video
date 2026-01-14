use core::fmt;

use alloc::vec;

use crate::{error::Error, video::decoder::Decoder};
use cyd_encoder::format::{FormatHeader, mjpeg::MjpegHeader};
use display_interface::DisplayError;
use embedded_graphics::{
    Drawable,
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
};
use embedded_io::{ErrorType, Read, ReadExactError, Seek, SeekFrom};
use tjpgdec_rs::{JpegDecoder, MemoryPool, RECOMMENDED_POOL_SIZE};
extern crate alloc;

pub struct MjpegDecoder<R>
where
    R: Read + Seek,
{
    header: MjpegHeader,
    reader: BufReader<R, 1024>,
}

// 15K buffer to read compressed JPG 320x240 image plus pool
pub const DECODE_SIZE: usize = (15 * 1024) + RECOMMENDED_POOL_SIZE;

mod markers {
    pub const SOI: (u8, u8) = (0xFF, 0xD8);
    pub const EOI: (u8, u8) = (0xFF, 0xD9);
}

impl<R> MjpegDecoder<R>
where
    R: Read + Seek,
{
    fn read_jpeg<'a>(
        &mut self,
        jpeg_data: &'a mut [u8],
    ) -> Result<&'a [u8], ReadExactError<R::Error>> {
        // First, find the SOI marker
        let mut prev_byte = 0u8;
        loop {
            let mut byte = [0u8];
            self.reader.read_exact(&mut byte)?;

            if prev_byte == markers::SOI.0 && byte[0] == markers::SOI.1 {
                break;
            }
            prev_byte = byte[0];
        }

        // Start copying data into jpeg_data, beginning with SOI
        jpeg_data[0] = markers::SOI.0;
        jpeg_data[1] = markers::SOI.1;
        let mut pos = 2;

        prev_byte = markers::SOI.1;

        // Copy data until we find EOI
        loop {
            let mut byte = [0u8];
            self.reader.read_exact(&mut byte)?;

            jpeg_data[pos] = byte[0];
            pos += 1;

            if prev_byte == markers::EOI.0 && byte[0] == markers::EOI.1 {
                break;
            }
            prev_byte = byte[0];
        }

        Ok(&jpeg_data[..pos])
    }
}

impl<R, D> Decoder<R, D, 1, MjpegHeader, { DECODE_SIZE }> for MjpegDecoder<R>
where
    R: Read + Seek,
    D: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    type DecoderError = tjpgdec_rs::Error;
    type ImageDrawable<'a> = JpegDrawable<'a>;

    fn new(mut reader: R) -> Result<Self, ReadExactError<R::Error>> {
        let mut buffer = [0u8; 1];
        reader.read_exact(&mut buffer)?;
        let header = MjpegHeader::parse(&buffer);

        Ok(Self {
            header,
            reader: BufReader::<_, 1024>::new(reader),
        })
    }

    fn header(&self) -> &MjpegHeader {
        &self.header
    }

    fn decode_into<'a>(
        &mut self,
        buffer: &'a mut [u8; DECODE_SIZE],
    ) -> Result<Self::ImageDrawable<'a>, Error<R::Error, Self::DecoderError>> {
        let [pool_buffer, decode_buffer] = buffer
            .get_disjoint_mut([0..RECOMMENDED_POOL_SIZE, RECOMMENDED_POOL_SIZE..DECODE_SIZE])
            .unwrap();
        let jpeg_data = match self.read_jpeg(decode_buffer) {
            Ok(data) => data,
            Err(ReadExactError::UnexpectedEof) => {
                self.reader
                    .seek(SeekFrom::Start(MjpegHeader::header_size() as u64))
                    .map_err(|e| Error::ReadError(e))?;
                return Err(Error::LoopEof);
            }
            Err(e) => return Err(Error::ReadExactError(e)),
        };
        JpegDrawable::new(pool_buffer, jpeg_data)
    }

    fn render<'a>(
        &'a self,
        image: Image<Self::ImageDrawable<'a>>,
        display: &mut D,
    ) -> Result<(), Error<R::Error, Self::DecoderError>> {
        image
            .draw(&mut display.color_converted())
            .map_err(Error::DisplayError)?;
        Ok(())
    }
}

pub struct JpegDrawable<'a> {
    jpeg_data: &'a [u8],
    decoder: JpegDecoder<'a>,
}

impl<'a> JpegDrawable<'a> {
    fn new<E>(
        pool_buffer: &'a mut [u8],
        jpeg_data: &'a [u8],
    ) -> Result<Self, Error<E, tjpgdec_rs::Error>>
    where
        E: fmt::Debug,
    {
        let mut pool = MemoryPool::new(pool_buffer);
        let mut decoder = JpegDecoder::new();
        decoder
            .prepare(jpeg_data, &mut pool)
            .map_err(Error::DecodeErrors)?;
        Ok(Self { jpeg_data, decoder })
    }

    fn render<E>(&mut self) -> Result<(), Error<E, tjpgdec_rs::Error>>
    where
        E: fmt::Debug,
    {
        let mcu_size = self.decoder.mcu_buffer_size();
        let work_size = self.decoder.work_buffer_size();
        let mut mcu_buffer = vec![0i16; mcu_size];
        let mut work_buffer = vec![0u8; work_size];
        self.decoder
            .decompress(
                self.jpeg_data,
                0,
                &mut mcu_buffer,
                &mut work_buffer,
                &mut |_decoder, _bitmap, rect| {
                    // Process pixel data

                    Ok(true)
                },
            )
            .map_err(Error::DecodeErrors)?;
        Ok(())
    }
}

impl ImageDrawable for JpegDrawable<'_> {
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        //target.fill_contiguous(&self.bounding_box(), self.pixels())
        todo!()
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.draw(&mut target.translated(-area.top_left).clipped(area))
    }
}

impl OriginDimensions for JpegDrawable<'_> {
    fn size(&self) -> Size {
        Size::new(self.decoder.width() as u32, self.decoder.height() as u32)
    }
}

struct BufReader<R, const BUFFER_SIZE: usize> {
    buffer: [u8; BUFFER_SIZE],
    reader: R,
    pos: usize,    // Current position in buffer
    filled: usize, // Number of valid bytes in buffer
}

impl<R: Read, const BUFFER_SIZE: usize> BufReader<R, BUFFER_SIZE> {
    fn new(reader: R) -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            reader,
            pos: 0,
            filled: 0,
        }
    }

    /// Returns the number of bytes available in the internal buffer
    fn available(&self) -> usize {
        self.filled - self.pos
    }

    /// Fill the buffer by reading from the underlying reader
    fn fill_buf(&mut self) -> Result<(), R::Error> {
        if self.pos >= self.filled {
            // Buffer is exhausted, refill it
            self.filled = self.reader.read(&mut self.buffer)?;
            self.pos = 0;
        }
        Ok(())
    }
}

impl<R: Read, const BUFFER_SIZE: usize> ErrorType for BufReader<R, BUFFER_SIZE> {
    type Error = R::Error;
}

impl<R: Read, const BUFFER_SIZE: usize> Read for BufReader<R, BUFFER_SIZE> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // If the requested read is larger than our buffer, and our buffer is empty,
        // bypass buffering and read directly
        if buf.len() >= BUFFER_SIZE && self.available() == 0 {
            return self.reader.read(buf);
        }

        // Ensure we have data in our buffer
        self.fill_buf()?;

        // Copy from internal buffer to output buffer
        let available = self.available();
        if available == 0 {
            return Ok(0); // EOF
        }

        let to_copy = available.min(buf.len());
        buf[..to_copy].copy_from_slice(&self.buffer[self.pos..self.pos + to_copy]);
        self.pos += to_copy;

        Ok(to_copy)
    }
}

impl<R: Read + Seek, const BUFFER_SIZE: usize> Seek for BufReader<R, BUFFER_SIZE> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let pos = self.reader.seek(pos)?;
        self.pos = 0;
        self.filled = 0;
        Ok(pos)
    }
}
