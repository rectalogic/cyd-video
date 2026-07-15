mod esp_new_jpeg;
mod mjpeg;
mod render;

use crate::display::DisplayAsyncMutex;
pub use mjpeg::{MjpegDecoder, MjpegError};
pub use render::JpegDrawable;

#[embassy_executor::task]
pub async fn video_task(display: &'static DisplayAsyncMutex) {}
