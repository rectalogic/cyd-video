extern crate alloc;

use core::ops::ControlFlow;

use crate::{
    display::DisplayAsyncMutex,
    error::Error,
    player::{audio::AudioBuffers, demux::Demuxer, video::VideoFrames},
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

    let mut video_frames = Some(VideoFrames::new(frame_duration));
    let mut audio_buffers = Some(AudioBuffers::new());

    while video_frames.is_some() || audio_buffers.is_some() {
        if let Some(ref mut video) = video_frames
            && !decode_video(&mut demuxer, video).await?
            && let Some(video) = video_frames.take()
        {
            video.finish().await;
        }

        if let Some(ref mut audio) = audio_buffers
            && !decode_audio(&mut demuxer, audio).await?
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
) -> Result<bool, Error> {
    if let Some(audio_chunk) = demuxer.next_audio_chunk() {
        audio_buffers.demux(demuxer, audio_chunk?).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn decode_video<R: Read + Seek>(
    demuxer: &mut Demuxer<R>,
    video_frames: &mut VideoFrames,
) -> Result<bool, Error> {
    if let Some(video_chunk) = demuxer.next_video_chunk() {
        video_frames.demux(demuxer, video_chunk?).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
