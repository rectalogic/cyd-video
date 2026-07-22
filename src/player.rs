extern crate alloc;

use core::ops::ControlFlow;

use crate::{
    display::DisplayAsyncMutex,
    error::Error,
    player::{
        audio::{AudioBuffer, AudioBuffers},
        demux::Demuxer,
        video::{VideoBuffer, VideoFrames},
    },
    sdcard::SdCard,
    touch::TouchDetector,
};
pub use demux::DemuxError;
use embassy_futures::select::{Either, select};
use embedded_io::{Read, Seek};
use embedded_sdmmc::{ShortFileName, VolumeIdx};

pub mod audio;
mod buffers;
mod clock;
mod demux;
pub mod video;

#[cfg(feature = "embed-video")]
static EMBEDDED_VIDEO: &[u8] = include_bytes!(env!("EMBED_VIDEO"));

#[cfg(feature = "embed-video")]
pub async fn play_embedded(touch_detector: &TouchDetector) -> Result<(), Error> {
    log::info!("Play embedded");
    play(super::cursor::Cursor::new(EMBEDDED_VIDEO), touch_detector).await
}

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

    let mut video_frames = Some(VideoFrames::new(frame_duration));
    let mut audio_buffers = Some(AudioBuffers::new());

    loop {
        let mut audio_buffer_future = None;
        let mut video_buffer_future = None;

        if let Some(ref mut video_frames) = video_frames {
            video_buffer_future = Some(video_frames.get_buffer());
        }
        if let Some(ref mut audio_buffers) = audio_buffers {
            audio_buffer_future = Some(audio_buffers.get_buffer());
        }

        let (audio_buffer, video_buffer) = match (audio_buffer_future, video_buffer_future) {
            (Some(audio_b_f), Some(video_b_f)) => match select(audio_b_f, video_b_f).await {
                Either::First(audio_buffer) => (Some(audio_buffer), None),
                Either::Second(video_buffer) => (None, Some(video_buffer)),
            },
            (Some(audio_b_f), None) => (Some(audio_b_f.await), None),
            (None, Some(video_b_f)) => (None, Some(video_b_f.await)),
            (None, None) => break,
        };

        if let Some(video_buffer) = video_buffer
            && let Some(ref mut video) = video_frames
            && !decode_video(&mut demuxer, video, video_buffer).await?
            && let Some(video) = video_frames.take()
        {
            video.finish().await;
        }

        if let Some(audio_buffer) = audio_buffer
            && let Some(ref mut audio) = audio_buffers
            && !decode_audio(&mut demuxer, audio, audio_buffer).await?
            && let Some(audio) = audio_buffers.take()
        {
            audio.finish().await;
        }

        if let Some(ref video) = video_frames
            && video.frame_count() % 5 == 0
            && touch_detector.was_touched()
        {
            if let Some(video) = video_frames.take() {
                video.finish().await;
            }
            if let Some(audio) = audio_buffers.take() {
                audio.finish().await;
            }
            break;
        }
    }
    Ok(())
}

async fn decode_audio<R: Read + Seek>(
    demuxer: &mut Demuxer<R>,
    audio_buffers: &mut AudioBuffers,
    buffer: AudioBuffer,
) -> Result<bool, Error> {
    if let Some(audio_chunk) = demuxer.next_audio_chunk() {
        audio_buffers.demux(demuxer, audio_chunk?, buffer).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn decode_video<R: Read + Seek>(
    demuxer: &mut Demuxer<R>,
    video_frames: &mut VideoFrames,
    buffer: VideoBuffer,
) -> Result<bool, Error> {
    if let Some(video_chunk) = demuxer.next_video_chunk() {
        video_frames.demux(demuxer, video_chunk?, buffer).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
