use crate::{display::DisplayAsyncMutex, error::Error, player::buffers::Buffers};
use core::sync::atomic::{AtomicU32, Ordering};
pub use device::{AudioDevice, AudioError, Peripherals};
use embassy_time::{Duration, Instant};

mod device;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

pub static AUDIO_BUFFERS: Buffers<3> = Buffers::new();

static AUDIO_CLOCK: AtomicU32 = AtomicU32::new(0);

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
    let mut latency_ms = 0;
    AudioClock::store(0);

    loop {
        let buffer = AUDIO_BUFFERS.receive().await;
        if buffer.data.is_empty() {
            break; // End-of-stream signaled by empty buffer
        }
        let mut remaining = buffer.data.as_slice();
        while !remaining.is_empty() {
            let elapsed = match current_time {
                Some(time) => {
                    remaining = &remaining[audio_device.push(remaining).await?..];
                    (time.elapsed().as_millis() as u32).saturating_sub(latency_ms)
                }
                None => {
                    let old_audio_bytes = AudioDevice::DMA_SIZE - audio_device.available().await?;
                    latency_ms = AudioClock::audio_bytes_to_ms(old_audio_bytes);
                    remaining = &remaining[audio_device.push(remaining).await?..];
                    current_time = Some(Instant::now());
                    0
                }
            };
            AudioClock::store(elapsed);
        }
    }

    Ok(())
}

pub struct AudioClock;

impl AudioClock {
    fn store(ms: u32) {
        AUDIO_CLOCK.store(ms, Ordering::Relaxed);
    }

    pub fn time() -> Duration {
        Duration::from_millis(AUDIO_CLOCK.load(Ordering::Relaxed) as u64)
    }

    const fn audio_bytes_to_ms(bytes: usize) -> u32 {
        const BYTES_PER_MS: u32 =
            (SAMPLE_RATE * SAMPLE_CHANNELS as u32 * SAMPLE_DATA_FORMAT as u32 / 8) / 1000;
        bytes as u32 / BYTES_PER_MS
    }
}
