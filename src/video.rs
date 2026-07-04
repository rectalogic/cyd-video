use core::{convert::Infallible, fmt, ops::DerefMut};

use crate::{
    display::{CENTER, Display},
    error::Error,
    sdcard::SdCard,
    touch::TouchDetector,
    video::{mjpeg::MjpegDecoder, render::JpegDrawable},
};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{Read, Seek};
use embedded_sdmmc::ShortFileName;
use riffparse::{EmbeddedAdapter, RiffParser, avi, fourcc::Fourcc};

mod esp_new_jpeg;
pub mod mjpeg;
mod render;

mod tag {
    use super::Fourcc;
    pub const MJPG: Fourcc = Fourcc::new(*b"MJPG");
}

pub async fn play_directory(
    avi_dir: &str,
    sdcard: &mut SdCard,
    display: &mut Display<'_>,
    touch_detector: &TouchDetector,
) -> Result<(), Error<embedded_sdmmc::Error<embedded_sdmmc::SdCardError>, Infallible>> {
    log::info!("Loading dir {avi_dir}");
    sdcard
        .open_directory(avi_dir, async |directory| {
            const MAX_FILES: usize = 5;
            let mut filenames: [Option<ShortFileName>; MAX_FILES] = [None; _];
            let mut index: usize = 0;
            if let Err(e) = directory.iterate_dir(|entry| {
                if index < MAX_FILES
                    && !entry.attributes.is_directory()
                    && entry.name.extension() == b"AVI"
                {
                    log::info!("Found {}", entry.name);
                    filenames[index] = Some(entry.name);
                    index += 1;
                };
            }) {
                display.message(format_args!("directory {avi_dir} error: {e:?}"))
            };
            filenames.sort();

            let filenames_cycle = filenames.into_iter().flatten().cycle();
            for filename in filenames_cycle {
                log::info!("Playing {filename}");
                match directory.open_file_in_dir(filename, embedded_sdmmc::Mode::ReadOnly) {
                    Ok(file) => match play(file, display.deref_mut(), touch_detector).await {
                        Ok(_) => {}
                        Err(e) => display.message(format_args!("{e:?}")),
                    },
                    Err(e) => display.message(format_args!("{filename} error: {e:?}")),
                };
            }

            Ok(())
        })
        .await
}

async fn play<R, DT>(
    reader: R,
    display: &mut DT,
    touch_detector: &TouchDetector,
) -> Result<(), Error<Infallible, DT::Error>>
where
    R: Read + Seek,
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: fmt::Debug,
{
    let avi_parser = avi::AviParser::new(RiffParser::new(EmbeddedAdapter(reader)))?;
    let Some(video_stream) = avi_parser.find_best_stream::<avi::VideoStream>() else {
        log::error!("No video stream found");
        return Ok(());
    };
    let stream_id = video_stream.stream_id;
    if !matches!(video_stream.stream_header.fcc_handler, tag::MJPG) {
        log::error!(
            "Unsupported fourcc {:?}",
            video_stream.stream_header.fcc_handler
        );
        return Ok(());
    }

    let frame_duration = Duration::from_micros(avi_parser.avi_header.micro_sec_per_frame as u64);
    // 15K buffer to read compressed JPG 320x240 image
    const BUFFER_SIZE: usize = 15 * 1024;
    let mut buffer = [0u8; BUFFER_SIZE];

    display.clear(Rgb565::BLACK).expect("clear");
    let decoder = MjpegDecoder::new()?;
    let mut size = None;
    let mut start: Option<Instant> = None;
    for (count, chunk) in avi_parser.movi_chunks(stream_id).enumerate() {
        let chunk = chunk?;
        let jpeg_data = &mut buffer[..(chunk.data_size() as usize)];
        avi_parser.riff_parser().read_data(chunk, jpeg_data)?;
        let jpeg_size = match size {
            Some(size) => size,
            None => {
                let (w, h) = decoder.prepare(jpeg_data)?;
                *size.insert(Size::new(w as u32, h as u32))
            }
        };
        let drawable = JpegDrawable::new(&decoder, jpeg_size, jpeg_data);
        let image = Image::with_center(&drawable, CENTER);
        if let Some(start) = start {
            let elapsed = start.elapsed();
            if frame_duration > elapsed {
                Timer::after(frame_duration - elapsed).await;
            } else {
                log::warn!("lag {:?}", elapsed - frame_duration);
            }
        }
        start = Some(Instant::now());
        image.draw(display).map_err(Error::DisplayError)?;

        if count % 5 == 0 && touch_detector.was_touched() {
            display.clear(Rgb565::BLUE).expect("clear");
            return Ok(());
        }
    }
    Ok(())
}
