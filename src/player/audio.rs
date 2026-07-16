use crate::{display::DisplayAsyncMutex, error::Error, player::buffers::Buffers};
pub use output::{AudioError, AudioOutput, Peripherals};

mod output;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

pub static AUDIO_BUFFERS: Buffers<2> = Buffers::new();

#[embassy_executor::task]
pub async fn audio_task(mut output: AudioOutput, display: &'static DisplayAsyncMutex) {
    AUDIO_BUFFERS.init();
    loop {
        if let Err(e) = play(&mut output).await {
            display.lock().await.message(format_args!("{e:?}"))
        }
    }
}

async fn play(output: &mut AudioOutput) -> Result<(), Error> {
    let mut buffer = AUDIO_BUFFERS.receive().await;
    if buffer.data.is_empty() {
        return Ok(());
    }
    let mut remaining = buffer.data.len();
    loop {
        if remaining > 0 {
            let written = output
                .push(&buffer.data[(buffer.data.len() - remaining)..])
                .await?;
            remaining -= written;
            continue;
        }

        buffer = AUDIO_BUFFERS.receive().await;
        if buffer.data.is_empty() {
            break; // End of stream
        }

        remaining = buffer.data.len() - output.push(&buffer.data).await?;
    }

    // End-of-stream: fill remaining buffer with silence to avoid looping
    const SILENCE: [u8; 512] = [0u8; 512];
    let mut silence_pushed: usize = 0;
    while silence_pushed < AudioOutput::DMA_SIZE {
        let written = output.push(&SILENCE).await?;
        silence_pushed += written;
    }
    Ok(())
}
