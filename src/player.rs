extern crate alloc;

use core::ops::ControlFlow;

use crate::{
    display::DisplayAsyncMutex,
    error::Error,
    player::{audio::AudioBuffers, video::VideoFrames},
    sdcard::SdCard,
    touch::TouchDetector,
};
pub use demux::DemuxError;
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

    //XXX don't use can_send, use blocking send - or this is busy-wait
    // XXX buffer 2 video frames, send video first (will block on audio signal)
    let mut video_frame_count = 0;
    let mut has_audio = true;
    let mut has_video = true;
    while has_audio || has_video {
        while has_audio {
            if let Some(audio_chunk) = demuxer.next_audio_chunk() {
                AudioBuffers::demux(&mut demuxer, audio_chunk?).await?;
            } else {
                has_audio = false;
                AudioBuffers::finish().await;
            }
        }
        if has_video {
            if let Some(video_chunk) = demuxer.next_video_chunk() {
                VideoFrames::demux(
                    &mut demuxer,
                    video_chunk?,
                    video_frame_count * frame_duration,
                )
                .await?;
                video_frame_count += 1;
            } else {
                has_video = false;
                VideoFrames::finish().await;
            }
        }

        if video_frame_count % 5 == 0 && touch_detector.was_touched() {
            if has_video {
                VideoFrames::finish().await;
            }
            if has_audio {
                AudioBuffers::finish().await;
            }
            break;
        }
    }
    Ok(())
}
