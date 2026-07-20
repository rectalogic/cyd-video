mod esp_new_jpeg;
mod mjpeg;
mod render;

extern crate alloc;
use crate::{
    display::{CENTER, DisplayAsyncMutex},
    error::Error,
    player::{
        audio::AudioClock,
        buffers::{Buffer, Buffers},
    },
};
use core::ops::DerefMut;
use embassy_time::{Duration, Timer};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use mjpeg::MjpegDecoder;
pub use mjpeg::MjpegError;
use render::JpegDrawable;

pub static VIDEO_FRAMES: Buffers<1, VideoFrame<1>> = Buffers::new();

#[embassy_executor::task]
pub async fn video_task(display: &'static DisplayAsyncMutex) {
    VIDEO_FRAMES.init();
    loop {
        if let Err(e) = play(display).await {
            display.lock().await.message(format_args!("{e:?}"))
        }
    }
}

async fn play(display: &'static DisplayAsyncMutex) -> Result<(), Error> {
    display
        .lock()
        .await
        .deref_mut()
        .clear(Rgb565::BLACK)
        .map_err(Error::Display)?;
    let decoder = MjpegDecoder::new()?;
    let mut size = None;
    loop {
        let frame = VIDEO_FRAMES.receive().await;
        if frame.buffer.data.is_empty() {
            return Ok(());
        }
        let jpeg_size = match size {
            Some(size) => size,
            None => {
                let (w, h) = decoder.prepare(&frame.buffer.data)?;
                // This is first frame of a new video, wait for audio clock
                AudioClock::started().await;
                *size.insert(Size::new(w as u32, h as u32))
            }
        };

        let audio_time = AudioClock::time();
        if frame.timestamp >= audio_time {
            Timer::after(frame.timestamp - audio_time).await;
        } else {
            log::warn!(
                "Skipping late frame {:?} (time {:?})",
                frame.timestamp,
                audio_time
            );
            continue;
        }
        render(&decoder, jpeg_size, &frame, display).await?;
    }
}

async fn render<const SIZE: usize>(
    decoder: &MjpegDecoder,
    size: Size,
    frame: &VideoFrame<SIZE>,
    display: &'static DisplayAsyncMutex,
) -> Result<(), Error> {
    let drawable = JpegDrawable::new(decoder, size, &frame.buffer.data);
    let image = Image::with_center(&drawable, CENTER);
    let mut display_guard = display.lock().await;
    image
        .draw(display_guard.deref_mut().deref_mut())
        .map_err(Error::Display)
}

pub struct VideoFrame<const SIZE: usize> {
    timestamp: Duration,
    buffer: Buffer<SIZE>,
}

impl<const SIZE: usize> VideoFrame<SIZE> {
    pub fn new(timestamp: Duration, buffer: Buffer<SIZE>) -> Self {
        Self { timestamp, buffer }
    }
}
