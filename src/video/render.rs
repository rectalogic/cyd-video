use core::cell::Cell;

use embedded_graphics::{
    geometry::Point,
    image::ImageDrawable,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::*,
    primitives::Rectangle,
};

use crate::video::mjpeg::MjpegDecoder;

pub struct JpegDrawable<'data, 'decoder> {
    size: Size,
    jpeg_data: &'data [u8],
    decoder: &'decoder MjpegDecoder,
}

impl<'data, 'decoder> JpegDrawable<'data, 'decoder> {
    pub fn new(decoder: &'decoder MjpegDecoder, size: Size, jpeg_data: &'data [u8]) -> Self {
        Self {
            decoder,
            size,
            jpeg_data,
        }
    }
}

impl ImageDrawable for JpegDrawable<'_, '_> {
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let display_error: Cell<Option<D::Error>> = Cell::new(None);
        if let Err(err) = self.decoder.decode(self.jpeg_data, |block| {
            let target_rect = Rectangle::new(
                Point::new(block.x as i32, block.y as i32),
                Size::new(block.width as u32, block.height as u32),
            );
            let pixels = block
                .data
                .chunks_exact(2)
                .map(|pixel| Rgb565::from(RawU16::from(u16::from_le_bytes([pixel[0], pixel[1]]))));
            // We can't return custom errors from the output function
            // https://docs.rs/tjpgdec-rs/0.4.0/tjpgdec_rs/type.OutputCallback.html
            if let Err(e) = target.fill_contiguous(&target_rect, pixels) {
                display_error.set(Some(e));
                return false;
            }
            true
        }) {
            // Not sure how we can return an error here
            log::error!("jpeg decode error: {err:?}");
        }
        if let Some(e) = display_error.take() {
            return Err(e);
        }
        Ok(())
    }

    fn draw_sub_image<D>(
        &self,
        target: &mut D,
        area: &Rectangle,
    ) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        self.draw(&mut target.translated(-area.top_left).clipped(area))
    }
}

impl OriginDimensions for JpegDrawable<'_, '_> {
    fn size(&self) -> Size {
        self.size
    }
}
