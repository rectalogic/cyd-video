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

pub mod decoder;
#[cfg(feature = "mjpeg")]
pub mod mjpeg;
#[cfg(feature = "yuv")]
pub mod yuv;

const CENTER: Point = Point::new(320 / 2, 240 / 2);

pub fn play<R, DT, const HEADER_SIZE: usize, F, const DECODE_SIZE: usize, D>(
    reader: R,
    mut display: &mut DT,
) -> Result<(), Error<R::Error, D::DecoderError>>
where
    R: Read + Seek,
    DT: DrawTarget<Color = Rgb565, Error = DisplayError>,
    F: FormatHeader<HEADER_SIZE>,
    D: Decoder<R, DT, HEADER_SIZE, F, DECODE_SIZE>,
{
    let delay = Delay::new();
    let mut start: Option<Instant> = None;
    let mut decoder = D::new(reader)?;
    let frame_duration = Duration::from_micros((1000 * 1000) / decoder.header().fps() as u64);
    let mut buffer = [0u8; DECODE_SIZE];
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
