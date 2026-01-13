use crate::{error::Error, video::decoder::Decoder};
use cyd_encoder::format::{FormatHeader, rgb::RgbHeader};
use display_interface::DisplayError;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::Rgb565,
    prelude::*,
};
use embedded_io::{Read, ReadExactError, Seek, SeekFrom};

pub struct RgbDecoder<R> {
    header: RgbHeader,
    reader: R,
}

pub const DECODE_SIZE: usize = (RgbHeader::MAX_WIDTH * RgbHeader::MAX_HEIGHT) * 2;

impl<R, D> Decoder<R, D, 5, RgbHeader, { DECODE_SIZE }> for RgbDecoder<R>
where
    R: Read + Seek,
    D: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    type DecoderError = R::Error;
    type ImageDrawable<'a> = ImageRaw<'a, Rgb565>;

    fn new(mut reader: R) -> Result<Self, ReadExactError<R::Error>> {
        let mut buffer = [0u8; 5];
        reader.read_exact(&mut buffer)?;
        let header = RgbHeader::parse(&buffer);
        Ok(Self { header, reader })
    }

    fn header(&self) -> &RgbHeader {
        &self.header
    }

    fn decode_into<'a>(
        &mut self,
        buffer: &'a mut [u8; DECODE_SIZE],
    ) -> Result<Self::ImageDrawable<'a>, Error<R::Error, Self::DecoderError>> {
        let width = self.header.width() as u32;
        let height = self.header.height() as u32;
        let buffer = &mut buffer[..((width * height) * 2) as usize];
        match self.reader.read_exact(buffer) {
            Ok(_) => {}
            Err(ReadExactError::UnexpectedEof) => {
                self.reader
                    .seek(SeekFrom::Start(RgbHeader::header_size() as u64))
                    .map_err(Error::ReadError)?;
                return Err(Error::LoopEof);
            }
            Err(ReadExactError::Other(e)) => return Err(Error::ReadError(e)),
        }
        Ok(ImageRaw::<Rgb565>::new(
            &buffer[..((width * height) * 2) as usize],
            width,
        ))
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
