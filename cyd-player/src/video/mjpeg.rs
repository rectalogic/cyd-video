use crate::{error::Error, video::decoder::Decoder};
use alloc::vec::Vec;
use cyd_encoder::format::{FormatHeader, mjpeg::MjpegHeader};
use display_interface::DisplayError;
use embedded_graphics::{
    Drawable,
    image::{Image, ImageRaw},
    pixelcolor::{Rgb565, Rgb888},
    prelude::{DrawTarget, DrawTargetExt},
};
use embedded_io::{Read, ReadExactError, Seek, SeekFrom};
use zune_jpeg::{
    JpegDecoder,
    errors::DecodeErrors,
    zune_core::{
        bytestream::{ZByteIoError, ZByteReaderTrait, ZSeekFrom},
        colorspace::ColorSpace,
        options::DecoderOptions,
    },
};
extern crate alloc;

pub struct MjpegDecoder<R>
where
    R: Read + Seek,
{
    header: MjpegHeader,
    reader: ZBufferedReader<R, 8192>,
    options: DecoderOptions,
}

pub const DECODE_SIZE: usize = MjpegHeader::MAX_WIDTH * MjpegHeader::MAX_HEIGHT * 3;

impl<R, D> Decoder<R, D, 1, MjpegHeader, { DECODE_SIZE }> for MjpegDecoder<R>
where
    R: Read + Seek,
    D: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    type DecoderError = DecodeErrors;
    type ImageDrawable<'a> = ImageRaw<'a, Rgb888>;

    fn new(mut reader: R) -> Result<Self, ReadExactError<R::Error>> {
        let mut buffer = [0u8; 1];
        reader.read_exact(&mut buffer)?;
        let header = MjpegHeader::parse(&buffer);
        let options = DecoderOptions::new_cmd()
            .set_max_width(MjpegHeader::MAX_WIDTH)
            .set_max_height(MjpegHeader::MAX_HEIGHT)
            .set_strict_mode(false)
            .jpeg_set_out_colorspace(ColorSpace::RGB);
        Ok(Self {
            header,
            reader: ZBufferedReader::<_, 8192>::new(reader),
            options,
        })
    }

    fn header(&self) -> &MjpegHeader {
        &self.header
    }

    fn decode_into<'a>(
        &mut self,
        buffer: &'a mut [u8; DECODE_SIZE],
    ) -> Result<ImageRaw<'a, Rgb888>, Error<R::Error, Self::DecoderError>> {
        let mut decoder = JpegDecoder::new_with_options(&mut self.reader, self.options);
        match decoder.decode_into(buffer) {
            Ok(()) => {}
            Err(DecodeErrors::IoErrors(ZByteIoError::NotEnoughBytes(_, _))) => {
                self.reader
                    .z_seek(ZSeekFrom::Start(MjpegHeader::header_size() as u64))
                    .map_err(|e| Error::DecodeErrors(DecodeErrors::IoErrors(e)))?;
                return Err(Error::LoopEof);
            }
            Err(e) => return Err(Error::DecodeErrors(e)),
        }
        let info = decoder
            .info()
            .ok_or(Error::DecodeErrors(DecodeErrors::FormatStatic(
                "no decoder info",
            )))?;
        Ok(ImageRaw::<Rgb888>::new(
            &buffer[..(info.width * info.height * 3) as usize],
            info.width as u32,
        ))
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

struct ZBufferedReader<R, const BUFFER_SIZE: usize = 8192> {
    inner: R,
    buffer: [u8; BUFFER_SIZE],
    pos: usize,
    filled: usize,
    stream_pos: u64,
}

impl<R: Read, const BUFFER_SIZE: usize> ZBufferedReader<R, BUFFER_SIZE> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: [0u8; BUFFER_SIZE],
            pos: 0,
            filled: 0,
            stream_pos: 0,
        }
    }

    fn fill_buf(&mut self) -> Result<&[u8], ZByteIoError> {
        if self.pos >= self.filled {
            self.filled = self
                .inner
                .read(&mut self.buffer)
                .map_err(|_| ZByteIoError::Generic("Failed to read from stream"))?;
            self.pos = 0;
        }
        Ok(&self.buffer[self.pos..self.filled])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = (self.pos + amt).min(self.filled);
        self.stream_pos += amt as u64;
    }

    fn available(&self) -> usize {
        self.filled - self.pos
    }
}

impl<R: Read + Seek, const BUFFER_SIZE: usize> ZByteReaderTrait
    for ZBufferedReader<R, BUFFER_SIZE>
{
    fn read_byte_no_error(&mut self) -> u8 {
        if let Ok(buf) = self.fill_buf()
            && !buf.is_empty()
        {
            let byte = buf[0];
            self.consume(1);
            return byte;
        }
        0
    }

    fn read_exact_bytes(&mut self, buf: &mut [u8]) -> Result<(), ZByteIoError> {
        let mut offset = 0;
        while offset < buf.len() {
            let available = self.fill_buf()?;
            if available.is_empty() {
                return Err(ZByteIoError::NotEnoughBytes(buf.len(), offset));
            }
            let to_copy = available.len().min(buf.len() - offset);
            buf[offset..offset + to_copy].copy_from_slice(&available[..to_copy]);
            self.consume(to_copy);
            offset += to_copy;
        }
        Ok(())
    }

    fn read_const_bytes<const N: usize>(&mut self, buf: &mut [u8; N]) -> Result<(), ZByteIoError> {
        self.read_exact_bytes(buf)
    }

    fn read_const_bytes_no_error<const N: usize>(&mut self, buf: &mut [u8; N]) {
        let _ = self.read_exact_bytes(buf);
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<usize, ZByteIoError> {
        let mut total = 0;
        while total < buf.len() {
            let available = self.fill_buf()?;
            if available.is_empty() {
                break;
            }
            let to_copy = available.len().min(buf.len() - total);
            buf[total..total + to_copy].copy_from_slice(&available[..to_copy]);
            self.consume(to_copy);
            total += to_copy;
        }
        Ok(total)
    }

    fn peek_bytes(&mut self, buf: &mut [u8]) -> Result<usize, ZByteIoError> {
        if buf.len() > BUFFER_SIZE {
            return Err(ZByteIoError::NotEnoughBuffer(BUFFER_SIZE, buf.len()));
        }

        let mut total = 0;
        let initial_pos = self.pos;

        while total < buf.len() {
            let available = self.fill_buf()?;
            if available.is_empty() {
                break;
            }
            let to_copy = available.len().min(buf.len() - total);
            buf[total..total + to_copy].copy_from_slice(&available[..to_copy]);
            self.pos += to_copy;
            total += to_copy;
        }

        // Reset position without updating stream_pos
        self.pos = initial_pos;
        Ok(total)
    }

    fn peek_exact_bytes(&mut self, buf: &mut [u8]) -> Result<(), ZByteIoError> {
        let read = self.peek_bytes(buf)?;
        if read < buf.len() {
            Err(ZByteIoError::NotEnoughBytes(buf.len(), read))
        } else {
            Ok(())
        }
    }

    fn z_seek(&mut self, from: ZSeekFrom) -> Result<u64, ZByteIoError> {
        let seek_from = match from {
            ZSeekFrom::Start(pos) => SeekFrom::Start(pos),
            ZSeekFrom::End(pos) => SeekFrom::End(pos),
            ZSeekFrom::Current(offset) => {
                // Adjust for buffered data
                let buffer_offset = -(self.available() as i64);
                SeekFrom::Current(offset + buffer_offset)
            }
        };

        let new_pos = self
            .inner
            .seek(seek_from)
            .map_err(|_| ZByteIoError::SeekError("Seek operation failed"))?;

        // Invalidate buffer after seek
        self.pos = 0;
        self.filled = 0;
        self.stream_pos = new_pos;

        Ok(new_pos)
    }

    fn is_eof(&mut self) -> Result<bool, ZByteIoError> {
        let buf = self.fill_buf()?;
        Ok(buf.is_empty())
    }

    fn z_position(&mut self) -> Result<u64, ZByteIoError> {
        Ok(self.stream_pos)
    }

    fn read_remaining(&mut self, sink: &mut Vec<u8>) -> Result<usize, ZByteIoError> {
        let mut total = 0;
        loop {
            let available = self.fill_buf()?;
            if available.is_empty() {
                break;
            }
            sink.extend_from_slice(available);
            let len = available.len();
            self.consume(len);
            total += len;
        }
        Ok(total)
    }
}

impl<R: Read + Seek, const BUFFER_SIZE: usize> ZByteReaderTrait
    for &mut ZBufferedReader<R, BUFFER_SIZE>
{
    fn read_byte_no_error(&mut self) -> u8 {
        (*self).read_byte_no_error()
    }

    fn read_exact_bytes(&mut self, buf: &mut [u8]) -> Result<(), ZByteIoError> {
        (*self).read_exact_bytes(buf)
    }

    fn read_const_bytes<const N: usize>(&mut self, buf: &mut [u8; N]) -> Result<(), ZByteIoError> {
        (*self).read_const_bytes(buf)
    }

    fn read_const_bytes_no_error<const N: usize>(&mut self, buf: &mut [u8; N]) {
        (*self).read_const_bytes_no_error(buf)
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<usize, ZByteIoError> {
        (*self).read_bytes(buf)
    }

    fn peek_bytes(&mut self, buf: &mut [u8]) -> Result<usize, ZByteIoError> {
        (*self).peek_bytes(buf)
    }

    fn peek_exact_bytes(&mut self, buf: &mut [u8]) -> Result<(), ZByteIoError> {
        (*self).peek_exact_bytes(buf)
    }

    fn z_seek(&mut self, from: ZSeekFrom) -> Result<u64, ZByteIoError> {
        (*self).z_seek(from)
    }

    fn is_eof(&mut self) -> Result<bool, ZByteIoError> {
        (*self).is_eof()
    }

    fn z_position(&mut self) -> Result<u64, ZByteIoError> {
        (*self).z_position()
    }

    fn read_remaining(&mut self, sink: &mut Vec<u8>) -> Result<usize, ZByteIoError> {
        (*self).read_remaining(sink)
    }
}
