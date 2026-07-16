extern crate alloc;

use core::ops::ControlFlow;

use crate::{
    display::DisplayAsyncMutex, error::Error, player::video::VIDEO_BUFFERS, sdcard::SdCard,
    touch::TouchDetector,
};
pub use demux::DemuxError;
use embassy_time::{Instant, Timer};
use embedded_io::{Read, Seek};
use embedded_sdmmc::{ShortFileName, VolumeIdx};

pub mod audio;
mod buffers;
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
        if index < MAX_FILES
            && !entry.attributes.is_directory()
            && entry.name.extension() == b"AVI"
            && &entry.name.base_name()[0..2] != b"._"
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
            Ok(file) => match play(file, touch_detector).await {
                Ok(_) => {}
                Err(e) => display.lock().await.message(format_args!("{e:?}")),
            },
            Err(e) => display
                .lock()
                .await
                .message(format_args!("{filename} error: {e:?}")),
        };
    }

    Ok(())
}

async fn play<R>(reader: R, touch_detector: &TouchDetector) -> Result<(), Error>
where
    R: Read + Seek,
{
    let mut demuxer = demux::Demuxer::new(reader)?;
    let frame_duration = demuxer.frame_duration();

    let mut start: Option<Instant> = None;
    let mut count = 0;
    while let Some(chunk) = demuxer.next_video_chunk() {
        let mut buffer = VIDEO_BUFFERS.get_recycled().await;
        demuxer.read_chunk_data(chunk?, &mut buffer.data)?;

        if let Some(start) = start {
            let elapsed = start.elapsed();
            if frame_duration > elapsed {
                Timer::after(frame_duration - elapsed).await;
            } else {
                log::warn!("lag {:?}", elapsed - frame_duration);
            }
        }
        VIDEO_BUFFERS.send(buffer).await;
        start = Some(Instant::now());

        if count % 5 == 0 && touch_detector.was_touched() {
            break;
        }
        count += 1;
    }

    // Send empty buffer
    let mut buffer = VIDEO_BUFFERS.get_recycled().await;
    buffer.data.clear();
    VIDEO_BUFFERS.send(buffer).await;
    Ok(())
}
