use crate::{display::DisplayAsyncMutex, error::Error, player::buffers::Buffers};
pub use device::{AudioDevice, AudioError, AudioOutput, Peripherals};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};

mod device;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

const SILENCE: [u8; 512] = [0u8; 512];
// 512 bytes / 32,000 bytes/sec ≈ 16ms → use approx 10ms for margin.
const FEED_DEADLINE_MS: u64 = ((SILENCE.len() as f64
    / (SAMPLE_RATE as f64 * (SAMPLE_DATA_FORMAT as f64 / 8.0)))
    * 0.6) as u64;

pub static AUDIO_BUFFERS: Buffers<2> = Buffers::new();

#[embassy_executor::task]
pub async fn audio_task(audio_peripherals: Peripherals, display: &'static DisplayAsyncMutex) {
    AUDIO_BUFFERS.init();

    let audio_device = match AudioDevice::new(audio_peripherals) {
        Ok(audio_device) => audio_device,
        Err(e) => display
            .lock()
            .await
            .message(format_args!("Audio error: {e:?}")),
    };

    let mut audio_output = match audio_device.output() {
        Ok(audio_output) => audio_output,
        Err(e) => display
            .lock()
            .await
            .message(format_args!("Audio output error: {e:?}")),
    };

    if let Err(e) = audio_output.fill_silence(&SILENCE).await {
        display
            .lock()
            .await
            .message(format_args!("Audio prefill error: {e:?}"))
    }

    loop {
        if let Err(e) = play(&mut audio_output).await {
            display
                .lock()
                .await
                .message(format_args!("Audio play error: {e:?}"))
        }
    }
}

async fn play(output: &mut AudioOutput) -> Result<(), Error> {
    loop {
        match select(
            AUDIO_BUFFERS.receive(),
            Timer::after(Duration::from_millis(FEED_DEADLINE_MS)),
        )
        .await
        {
            Either::First(buffer) => {
                // Got real audio data
                if buffer.data.is_empty() {
                    break; // End-of-stream signaled by empty buffer
                }
                let mut remaining = buffer.data.as_slice();
                while !remaining.is_empty() {
                    remaining = &remaining[output.push(remaining).await?..];
                }
            }
            Either::Second(_) => {
                // Deadline fired — DMA is about to run dry, push silence
                output.push(&SILENCE).await?;
            }
        }
    }

    // End-of-stream: cap with silence (same as before)
    output.fill_silence(&SILENCE).await?;
    Ok(())
}
