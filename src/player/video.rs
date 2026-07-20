mod esp_new_jpeg;
mod mjpeg;
mod render;

extern crate alloc;
use crate::{
    display::{CENTER, DisplayAsyncMutex},
    error::Error,
    player::{
        buffers::{Buffer, Buffers},
        clock::Clock,
        demux::Demuxer,
    },
};
use core::{fmt, ops::DerefMut};
use embassy_time::{Duration, Timer};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_io::{Read, Seek};
use mjpeg::MjpegDecoder;
pub use mjpeg::MjpegError;
use render::JpegDrawable;
use riffparse::{Chunk, Riff};

const BUFFER_COUNT: usize = 2;
static VIDEO_FRAMES: Buffers<BUFFER_COUNT, VideoFrame> = Buffers::new();
pub type VideoBuffer = Buffer<BUFFER_COUNT>;

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
    let mut initialized = None;
    loop {
        let frame = VIDEO_FRAMES.receive().await;
        if frame.buffer.data.is_empty() {
            return Ok(());
        }
        let (jpeg_size, clock) = match initialized {
            Some(initialized) => initialized,
            None => {
                let (w, h) = decoder.prepare(&frame.buffer.data)?;
                // This is first frame of a new video, wait for clock
                *initialized.insert((Size::new(w as u32, h as u32), Clock::started().await))
            }
        };

        let time = clock.elapsed();
        let timestamp = frame.timestamp();
        if time < timestamp {
            Timer::after(timestamp - time).await;
        } else if time > timestamp + frame.frame_duration {
            log::warn!("Skipping late frame {:?} (time {:?})", frame, time);
            continue;
        }
        render(&decoder, jpeg_size, &frame, display).await?;
    }
}

async fn render(
    decoder: &MjpegDecoder,
    size: Size,
    frame: &VideoFrame,
    display: &'static DisplayAsyncMutex,
) -> Result<(), Error> {
    let drawable = JpegDrawable::new(decoder, size, &frame.buffer.data);
    let image = Image::with_center(&drawable, CENTER);
    let mut display_guard = display.lock().await;
    image
        .draw(display_guard.deref_mut().deref_mut())
        .map_err(Error::Display)
}

pub struct VideoFrames {
    frame_count: u32,
    frame_duration: Duration,
}

impl VideoFrames {
    pub fn new(frame_duration: Duration) -> Self {
        Self {
            frame_count: 0,
            frame_duration,
        }
    }

    pub async fn get_buffer(&self) -> VideoBuffer {
        VIDEO_FRAMES.get_recycled().await
    }

    pub async fn demux<R: Read + Seek>(
        &mut self,
        demuxer: &mut Demuxer<R>,
        chunk: Riff<Chunk>,
        mut buffer: VideoBuffer,
    ) -> Result<(), Error> {
        demuxer.read_chunk_data(chunk, &mut buffer.data)?;
        let frame = VideoFrame::new(self.frame_count, self.frame_duration, buffer);
        VIDEO_FRAMES.send(frame).await;
        self.frame_count += 1;
        Ok(())
    }

    pub async fn finish(self) {
        let mut buffer = VIDEO_FRAMES.get_recycled().await;
        buffer.data.clear();
        let frame = VideoFrame::new(0, Duration::MIN, buffer);
        VIDEO_FRAMES.send(frame).await;
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}

struct VideoFrame {
    count: u32,
    frame_duration: Duration,
    buffer: VideoBuffer,
}

impl VideoFrame {
    fn new(count: u32, frame_duration: Duration, buffer: VideoBuffer) -> Self {
        Self {
            count,
            frame_duration,
            buffer,
        }
    }

    fn timestamp(&self) -> Duration {
        self.count * self.frame_duration
    }
}

impl fmt::Debug for VideoFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let timestamp = self.timestamp();
        f.debug_struct("VideoFrame")
            .field("count", &self.count)
            .field("timestamp", &timestamp)
            .field("end_timestamp", &(timestamp + self.frame_duration))
            .finish()
    }
}
