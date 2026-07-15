extern crate alloc;
use bytes::BytesMut;
use core::ops::{ControlFlow, DerefMut};

use crate::{
    display::{CENTER, DisplayAsyncMutex},
    error::Error,
    player::video::{JpegDrawable, MjpegDecoder},
    sdcard::SdCard,
    touch::TouchDetector,
};
pub use demux::DemuxError;
use embassy_time::{Instant, Timer};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{Read, Seek};
use embedded_sdmmc::{ShortFileName, VolumeIdx};

#[cfg(esp32s3)]
pub mod audio;
mod demux;
pub mod video;

pub async fn play_directory(
    avi_dir: &str,
    sdcard: &mut SdCard,
    display: &'static DisplayAsyncMutex,
    touch_detector: &TouchDetector,
) -> Result<(), Error> {
    log::info!("Loading dir {avi_dir}");
    let volume = sdcard.open_volume(VolumeIdx(0))?;
    let root_directory = volume.open_root_dir()?;
    let directory = root_directory.open_dir(avi_dir)?;

    const MAX_FILES: usize = 5;
    let mut filenames: [Option<ShortFileName>; MAX_FILES] = [None; _];
    let mut index: usize = 0;
    directory.iterate_dir(|entry| {
        if index < MAX_FILES && !entry.attributes.is_directory() && entry.name.extension() == b"AVI"
        {
            log::info!("Found {}", entry.name);
            filenames[index] = Some(entry.name);
            index += 1;
        };
        ControlFlow::Continue(())
    })?;
    filenames.sort();

    let filenames_cycle = filenames.into_iter().flatten().cycle();
    for filename in filenames_cycle {
        log::info!("Playing {filename}");
        match directory.open_file_in_dir(filename, embedded_sdmmc::Mode::ReadOnly) {
            Ok(file) => match play(file, display, touch_detector).await {
                Ok(_) => {}
                Err(e) => display.lock().await.message(format_args!("{e:?}")),
            },
            Err(e) => display.lock().await.message(format_args!("{filename} error: {e:?}")),
        };
    }

    Ok(())
}

async fn play<R>(
    reader: R,
    display: &'static DisplayAsyncMutex,
    touch_detector: &TouchDetector,
) -> Result<(), Error>
where
    R: Read + Seek,
{
    let mut demuxer = demux::Demuxer::new(reader)?;
    let frame_duration = demuxer.frame_duration();

    // 15K buffer to read compressed JPG 320x240 image
    const BUFFER_SIZE: usize = 15 * 1024;
    let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);

    display.lock().await.deref_mut().clear(Rgb565::BLACK).map_err(Error::Display)?;
    let decoder = MjpegDecoder::new()?;
    let mut size = None;
    let mut start: Option<Instant> = None;
    let mut count = 0;
    while let Some(chunk) = demuxer.next_video_chunk() {
        demuxer.read_chunk_data(chunk?, &mut buffer)?;
        let jpeg_size = match size {
            Some(size) => size,
            None => {
                let (w, h) = decoder.prepare(&buffer)?;
                *size.insert(Size::new(w as u32, h as u32))
            }
        };
        let drawable = JpegDrawable::new(&decoder, jpeg_size, &buffer);
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
        let mut display_guard = display.lock().await;
        image.draw(display_guard.deref_mut().deref_mut()).map_err(Error::Display)?;

        if count % 5 == 0 && touch_detector.was_touched() {
            display_guard.clear(Rgb565::BLUE).expect("clear");
            return Ok(());
        }
        count += 1;
    }
    Ok(())
}
