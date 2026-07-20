extern crate alloc;

use core::ops::ControlFlow;

use crate::{
    display::DisplayAsyncMutex,
    error::Error,
    player::{
        audio::AUDIO_BUFFERS,
        video::{VIDEO_FRAMES, VideoFrame},
    },
    sdcard::SdCard,
    touch::TouchDetector,
};
pub use demux::DemuxError;
use embassy_time::Duration;
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
        while has_audio && AUDIO_BUFFERS.can_send() {
            if let Some(audio_chunk) = demuxer.next_audio_chunk() {
                let mut buffer = AUDIO_BUFFERS.get_recycled().await;
                demuxer.read_chunk_data(audio_chunk?, &mut buffer.data)?;
                AUDIO_BUFFERS.send(buffer).await;
            } else {
                has_audio = false;
                let mut buffer = AUDIO_BUFFERS.get_recycled().await;
                buffer.data.clear();
                AUDIO_BUFFERS.send(buffer).await;
            }
        }
        if has_video && VIDEO_FRAMES.can_send() {
            if let Some(video_chunk) = demuxer.next_video_chunk() {
                let mut buffer = VIDEO_FRAMES.get_recycled().await;
                demuxer.read_chunk_data(video_chunk?, &mut buffer.data)?;
                let frame = VideoFrame::new(video_frame_count * frame_duration, buffer);
                VIDEO_FRAMES.send(frame).await;
                video_frame_count += 1;
            } else {
                has_video = false;
                let mut buffer = VIDEO_FRAMES.get_recycled().await;
                buffer.data.clear();
                VIDEO_FRAMES
                    .send(VideoFrame::new(Duration::MIN, buffer))
                    .await;
            }
        }

        if video_frame_count % 5 == 0 && touch_detector.was_touched() {
            //XXX send empties if not yet sent
            break;
        }
    }
    Ok(())
}
