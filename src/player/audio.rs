use crate::{display::DisplayAsyncMutex, error::Error, player::buffers::Buffers};
use core::sync::atomic::{AtomicU32, Ordering};
pub use device::{AudioDevice, AudioError, Peripherals};
use embassy_time::Instant;

mod device;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

pub static AUDIO_BUFFERS: Buffers<3> = Buffers::new();

pub static AUDIO_CLOCK: AtomicU32 = AtomicU32::new(0);
const BYTES_PER_MS: u32 =
    (SAMPLE_RATE * SAMPLE_CHANNELS as u32 * SAMPLE_DATA_FORMAT as u32 / 8) / 1000;
const CLOCK_LATENCY_MS: u32 = (AudioDevice::DMA_SIZE as u32 / BYTES_PER_MS) / 2;

#[embassy_executor::task]
pub async fn audio_task(audio_peripherals: Peripherals, display: &'static DisplayAsyncMutex) {
    AUDIO_BUFFERS.init();

    let mut audio_device = match AudioDevice::new(audio_peripherals) {
        Ok(audio_device) => audio_device,
        Err(e) => display
            .lock()
            .await
            .message(format_args!("Audio error: {e:?}")),
    };

    loop {
        if let Err(e) = play(&mut audio_device).await {
            display
                .lock()
                .await
                .message(format_args!("Audio play error: {e:?}"))
        }
    }
}

async fn play(audio_device: &mut AudioDevice) -> Result<(), Error> {
    let mut current_time: Option<Instant> = None;
    AUDIO_CLOCK.store(0, Ordering::Relaxed);

    loop {
        let buffer = AUDIO_BUFFERS.receive().await;
        if buffer.data.is_empty() {
            break; // End-of-stream signaled by empty buffer
        }
        let mut remaining = buffer.data.as_slice();
        while !remaining.is_empty() {
            remaining = &remaining[audio_device.push(remaining).await?..];
            let elapsed = match current_time {
                Some(time) => time.elapsed().as_millis() as u32,
                None => {
                    current_time = Some(Instant::now());
                    0
                }
            };
            AUDIO_CLOCK.store(elapsed.saturating_sub(CLOCK_LATENCY_MS), Ordering::Relaxed);
        }
    }

    Ok(())
}
