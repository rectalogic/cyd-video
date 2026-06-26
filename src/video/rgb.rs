use crate::{display, error::Error, video::decoder::Decoder};
use core::{convert::Infallible, fmt};
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::Rgb565,
    prelude::*,
};

pub const MAX_ENCODED_SIZE: usize = (display::DISPLAY_WIDTH * display::DISPLAY_HEIGHT) as usize * 2;

pub struct RgbDecoder {
    width: u32,
}

impl RgbDecoder {
    pub fn new(width: u32) -> Self {
        Self { width }
    }
}

impl<D> Decoder<D> for RgbDecoder
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: fmt::Debug,
{
    type ImageDrawable<'a> = ImageRaw<'a, Rgb565>;

    fn decode_frame<'a>(
        &mut self,
        frame_buffer: &'a mut [u8],
        frame_size: usize,
    ) -> Result<Self::ImageDrawable<'a>, Error<Infallible, D::Error>> {
        Ok(ImageRaw::<Rgb565>::new(
            &frame_buffer[..frame_size],
            self.width,
        ))
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
