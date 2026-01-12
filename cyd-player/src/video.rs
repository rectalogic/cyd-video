use core::ops::DerefMut;

use crate::{error::Error, video::decoder::Decoder};
use cyd_encoder::format::FormatHeader;
use display_interface::DisplayError;
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{Read, Seek};
use esp_hal::{
    delay::Delay,
    time::{Duration, Instant},
};
use mjpeg::MjpegDecoder;

mod decoder;
mod mjpeg;

const CENTER: Point = Point::new(320 / 2, 240 / 2);

pub struct Video;

impl Video {
    pub fn play<R, D>(reader: &mut R, display: &mut D) -> Result<(), Error<R::Error>>
    where
        R: Read + Seek,
        D: DrawTarget<Color = Rgb565, Error = DisplayError>,
    {
        let delay = Delay::new();
        let mut start: Option<Instant> = None;
        let mut decoder = MjpegDecoder::new(reader)?;
        let frame_duration = Duration::from_micros((1000 * 1000) / decoder.header().fps() as u64);
        let mut buffer = [0u8; MjpegDecoder::<R>::DECODE_BUFFER_SIZE];
        loop {
            let pixels = match decoder.decode_into(&mut buffer) {
                Ok(pixels) => pixels,
                Err(Error::LoopEof) => {
                    continue;
                }
                Err(e) => return Err(e),
            };
            let image = Image::with_center(&pixels, CENTER);
            if let Some(start) = start {
                let elapsed = start.elapsed();
                if frame_duration > elapsed {
                    delay.delay(frame_duration - elapsed);
                } else {
                    log::warn!("lag {:?}", elapsed - frame_duration);
                }
            }
            start = Some(Instant::now());
            decoder.render(image, display.deref_mut())?;
        }
    }
}
