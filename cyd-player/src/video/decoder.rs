use core::fmt;

use crate::error::Error;
use cyd_encoder::format::FormatHeader;
use embedded_graphics::{
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::DrawTarget,
};
use embedded_io::{Read, ReadExactError, Seek};

pub trait Decoder<R, D, const HEADER_SIZE: usize, F, const DECODE_SIZE: usize>
where
    R: Read + Seek,
    D: DrawTarget<Color = Rgb565>,
    D::Error: fmt::Debug,
    F: FormatHeader<HEADER_SIZE>,
    Self: Sized,
{
    type DecoderError: fmt::Debug;
    type ImageDrawable<'a>: ImageDrawable + 'a;

    fn new(reader: R) -> Result<Self, ReadExactError<R::Error>>;

    fn header(&self) -> &F;

    #[allow(clippy::type_complexity)]
    fn decode_into<'a>(
        &mut self,
        buffer: &'a mut [u8; DECODE_SIZE],
    ) -> Result<Self::ImageDrawable<'a>, Error<R::Error, Self::DecoderError, D::Error>>;

    #[allow(clippy::type_complexity)]
    fn render<'a>(
        &'a self,
        image: Image<Self::ImageDrawable<'a>>,
        display: &mut D,
    ) -> Result<(), Error<R::Error, Self::DecoderError, D::Error>>;
}
