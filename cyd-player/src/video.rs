use core::{convert::Infallible, fmt, ops::DerefMut};

use crate::{display::CENTER, error::Error, touch::TouchDetector, video::decoder::Decoder};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{Read, Seek};
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
    pub const MJPG: Fourcc = Fourcc::new(*b"MJPG");
    pub const I420: Fourcc = Fourcc::new(*b"I420");
    pub const NONE: Fourcc = Fourcc::from_u32(0);
}

pub fn play<R, DT>(
    reader: R,
    display: &mut DT,
    touch_detector: &TouchDetector,
) -> Result<(), Error<Infallible, DT::Error>>
where
    R: Read + Seek,
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: fmt::Debug,
{
    let mut avi_parser = avi::AviParser::new(RiffParser::new(EmbeddedAdapter(reader)))?;
    let Some(video_stream) = avi_parser.find_best_stream::<avi::VideoStream>() else {
        log::error!("No video stream found");
        return Ok(());
    };
    let stream_id = video_stream.stream_id;
    match video_stream.stream_header.fcc_handler {
        tag::MJPG => decoder_play::<_, _, _, { mjpeg::MAX_ENCODED_SIZE }>(
            mjpeg::MjpegDecoder::new(),
            display,
            touch_detector,
            &mut avi_parser,
            stream_id,
        ),
        tag::I420 => decoder_play::<_, _, _, { rgb::MAX_ENCODED_SIZE }>(
            rgb::RgbDecoder::new(avi_parser.avi_header.width),
            display,
            touch_detector,
            &mut avi_parser,
            stream_id,
        ),
        tag::NONE => decoder_play::<_, _, _, { yuv::MAX_ENCODED_SIZE }>(
            yuv::YuvDecoder::new(avi_parser.avi_header.width, avi_parser.avi_header.height),
            display,
            touch_detector,
            &mut avi_parser,
            stream_id,
        ),
        fcc => {
            log::error!("Unsupported fourcc {fcc:?}");
            Ok(())
        }
    }
}

fn decoder_play<D, DT, R, const BUFFER_SIZE: usize>(
    mut decoder: D,
    mut display: &mut DT,
    touch_detector: &TouchDetector,
    avi_parser: &mut avi::AviParser<EmbeddedAdapter<R>>,
    stream_id: Fourcc,
) -> Result<(), Error<Infallible, DT::Error>>
where
    DT: DrawTarget<Color = Rgb565>,
    D: Decoder<DT>,
    DT::Error: fmt::Debug,
    R: Read + Seek,
{
    let frame_duration = Duration::from_micros(avi_parser.avi_header.micro_sec_per_frame as u64);
    let mut buffer = [0u8; BUFFER_SIZE];

    display.clear(Rgb565::BLACK).expect("clear");
    let delay = Delay::new();
    let mut start: Option<Instant> = None;
    for (count, chunk) in avi_parser.movi_chunks(stream_id).enumerate() {
        let chunk = chunk?;
        let size = chunk.data_size() as usize;
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

        if count % 5 == 0 && touch_detector.was_touched() {
            display.clear(Rgb565::BLUE).expect("clear");
            return Ok(());
        }
    }
    Ok(())
}
