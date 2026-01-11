use crate::error::Error;
use cyd_encoder::{HEADER_SIZE, parse_header};
use display_interface::DisplayError;
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{Read, ReadExactError, Seek};
use esp_hal::{
    delay::Delay,
    time::{Duration, Instant},
};
use mjpeg::MjpegDecoder;
use zune_jpeg::errors::DecodeErrors;

mod mjpeg;

const MAX_WIDTH: usize = 320;
const MAX_HEIGHT: usize = 240;
const CENTER: Point = Point::new((MAX_WIDTH / 2) as i32, (MAX_HEIGHT / 2) as i32);

pub struct Video {
    fps: u8,
}

impl Video {
    pub fn new<R: Read>(reader: &mut R) -> Result<Self, ReadExactError<R::Error>> {
        let mut header = [0u8; HEADER_SIZE];
        reader.read_exact(&mut header)?;
        let fps = parse_header(&header);
        Ok(Self { fps })
    }

    pub fn play<R, D>(&mut self, reader: &mut R, display: &mut D) -> Result<(), Error<R::Error>>
    where
        R: Read + Seek,
        D: DrawTarget<Color = Rgb565, Error = DisplayError>,
    {
        let delay = Delay::new();
        let frame_duration = Duration::from_micros((1000 * 1000) / self.fps as u64);
        let mut start: Option<Instant> = None;
        let mut decoder = MjpegDecoder::new(reader);
        let mut buffer = [0u8; MAX_WIDTH * MAX_HEIGHT * 3];
        loop {
            let pixels = match decoder.decode_into(&mut buffer) {
                Ok(pixels) => pixels,
                Err(Error::DecodeErrors(DecodeErrors::ExhaustedData)) => {
                    // Loop video
                    decoder.seek(HEADER_SIZE as u64)?;
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
            image
                .draw(&mut display.color_converted())
                .map_err(Error::DisplayError)?;
        }
    }
}
