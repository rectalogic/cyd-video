use core::{convert::Infallible, fmt};

use crate::error::Error;
use embedded_graphics::{
    image::{Image, ImageDrawable},
    pixelcolor::Rgb565,
    prelude::DrawTarget,
};

pub trait Decoder<D>
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: fmt::Debug,
    Self: Sized,
{
    const MAX_ENCODED_SIZE: usize;
    type ImageDrawable<'a>: ImageDrawable + 'a;

    #[allow(clippy::type_complexity)]
    fn decode_frame<'a>(
        &mut self,
        frame_buffer: &'a mut [u8],
        frame_size: usize,
    ) -> Result<Self::ImageDrawable<'a>, Error<Infallible, D::Error>>;

    #[allow(clippy::type_complexity)]
    fn render<'a>(
        &'a self,
        image: Image<Self::ImageDrawable<'a>>,
        display: &mut D,
    ) -> Result<(), Error<Infallible, D::Error>>;
}
