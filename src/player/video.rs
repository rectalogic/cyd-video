mod esp_new_jpeg;
mod mjpeg;
mod render;

extern crate alloc;
use crate::{
    display::{CENTER, DisplayAsyncMutex},
    error::Error,
    player::buffers::Buffers,
};
use core::ops::DerefMut;
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use mjpeg::MjpegDecoder;
pub use mjpeg::MjpegError;
use render::JpegDrawable;

pub static VIDEO_BUFFERS: Buffers<1> = Buffers::new();

#[embassy_executor::task]
pub async fn video_task(display: &'static DisplayAsyncMutex) {
    VIDEO_BUFFERS.init();
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
        let buffer = VIDEO_BUFFERS.receive().await;
        if buffer.data.is_empty() {
            return Ok(());
        }

        let jpeg_size = match size {
            Some(size) => size,
            None => {
                let (w, h) = decoder.prepare(&buffer.data)?;
                *size.insert(Size::new(w as u32, h as u32))
            }
        };
        let drawable = JpegDrawable::new(&decoder, jpeg_size, &buffer.data);
        let image = Image::with_center(&drawable, CENTER);
        let mut display_guard = display.lock().await;
        image
            .draw(display_guard.deref_mut().deref_mut())
            .map_err(Error::Display)?;
    }
}
