use core::{fmt, ops::DerefMut};

use crate::{display::CENTER, error::Error, touch::TouchDetector, video::decoder::Decoder};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{ErrorType, Read, Seek};
use esp_hal::{
    delay::Delay,
    time::{Duration, Instant},
};
use riffparse::{EmbeddedAdapter, RiffParser, avi, fourcc::Fourcc};

pub mod decoder;
pub mod mjpeg;
pub mod rgb;
pub mod yuv;

mod tag {
    use super::Fourcc;
    const MJPG: Fourcc = Fourcc::new(*b"MJPG");
    const I420: Fourcc = Fourcc::new(*b"I420");
    const NONE: Fourcc = Fourcc::from_u32(0);
}

const MAX_ENCODED_SIZE: usize = mjpeg::MjpegDecoder::MAX_ENCODED_SIZE
    .max(yuv::YuvDecoder::MAX_ENCODED_SIZE)
    .max(rgb::RgbDecoder::MAX_ENCODED_SIZE);

fn create_decoder(video_stream: &avi::VideoStream, avi_parser: &avi::AviParser) -> impl Decoder {
    match video_stream.stream_header.fcc_type {
        tag::MJPG => mjpeg::MjpegDecoder::new(),
        tag::I420 => rgb::RgbDecoder::new(avi_parser.avi_header.width),
        tag::NONE => {
            yuv::YuvDecoder::new(avi_parser.avi_header.width, avi_parser.avi_header.height)
        }
    };
}

#[allow(clippy::type_complexity)]
pub fn play<R, DT>(
    reader: R,
    mut display: &mut DT,
    touch_detector: &TouchDetector,
) -> Result<(), Error<R::Error, DT::Error>>
where
    R: Read + Seek + ErrorType,
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: fmt::Debug,
{
    let avi_parser = avi::AviParser::new(RiffParser::new(EmbeddedAdapter(reader)))?;
    let Some(video_stream) = avi_parser.find_best_stream::<avi::VideoStream>() else {
        log::error!("No video stream found");
        return Ok(());
    };

    let mut decoder = create_decoder(&video_stream, &avi_parser);
    let frame_duration = Duration::from_micros(avi_parser.avi_header.micro_sec_per_frame as u64);
    let mut buffer = [0u8; MAX_ENCODED_SIZE];

    display.clear(Rgb565::BLACK).expect("clear");
    let delay = Delay::new();
    let mut start: Option<Instant> = None;
    for (count, chunk) in avi_parser.movi_chunks(video_stream.stream_id).enumerate() {
        let size = chunk.data_size();
        avi_parser
            .riff_parser()
            .read_data(chunk, &mut buffer[..size])?;
        let pixels = decoder.decode_frame(&mut buffer, size)?;
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

        if count % 5 == 0 {
            if touch_detector.was_touched() {
                display.clear(Rgb565::BLUE).expect("clear");
                return Ok(());
            }
        }
    }
    Ok(())
}
