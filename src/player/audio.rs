use crate::{
    display::DisplayAsyncMutex,
    error::Error,
    player::{
        buffers::{Buffer, Buffers},
        clock::Clock,
        demux::Demuxer,
    },
};
use core::fmt;
pub use device::{AudioDevice, AudioError, Peripherals};
use embassy_time::Duration;
use embedded_io::{Read, Seek};
use riffparse::{Chunk, Riff};

mod device;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

const BUFFER_COUNT: usize = 5;
static AUDIO_BUFFERS: Buffers<BUFFER_COUNT, Buffer<BUFFER_COUNT>> = Buffers::new();
pub type AudioBuffer = Buffer<BUFFER_COUNT>;

#[embassy_executor::task]
pub async fn audio_task(audio_peripherals: Peripherals, display: &'static DisplayAsyncMutex) {
    AUDIO_BUFFERS.init();

    let mut audio_device = match AudioDevice::new(audio_peripherals) {
        Ok(audio_device) => audio_device,
        Err(e) => display_error(display, format_args!("Audio error: {e:?}")).await,
    };

    loop {
        if let Err(e) = play_silence(&mut audio_device).await {
            display_error(display, format_args!("Audio play silence error: {e:?}")).await
        }
        if let Err(e) = play(&mut audio_device).await {
            display_error(display, format_args!("Audio play error: {e:?}")).await
        }
    }
}

async fn play_silence(audio_device: &mut AudioDevice) -> Result<(), Error> {
    const SILENCE: [u8; 512] = [0; _];
    const TIMEOUT: Duration = Duration::from_millis(audio_bytes_to_ms(SILENCE.len()) as u64 / 2);
    audio_device.push(&SILENCE).await?;

    while AUDIO_BUFFERS.receive_timeout(TIMEOUT).await.is_err() {
        audio_device.push(&SILENCE).await?;
    }
    Ok(())
}

async fn play(audio_device: &mut AudioDevice) -> Result<(), Error> {
    let mut clock_started = false;

    loop {
        let buffer = AUDIO_BUFFERS.receive().await;
        if buffer.data.is_empty() {
            break; // End-of-stream signaled by empty buffer
        }
        let mut remaining = buffer.data.as_slice();
        while !remaining.is_empty() {
            if !clock_started {
                let old_audio_bytes = AudioDevice::DMA_SIZE - audio_device.available().await?;
                let latency_ms = audio_bytes_to_ms(old_audio_bytes);
                remaining = &remaining[audio_device.push(remaining).await?..];
                Clock::start(Duration::from_millis(latency_ms as u64));
                clock_started = true;
            } else {
                remaining = &remaining[audio_device.push(remaining).await?..];
            }
        }
    }

    Ok(())
}

async fn display_error(display: &'static DisplayAsyncMutex, message: fmt::Arguments<'_>) -> ! {
    display.lock().await.message(message)
}

#[derive(Default)]
pub struct AudioBuffers {
    _private: (),
}

impl AudioBuffers {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub async fn get_buffer(&self) -> AudioBuffer {
        AUDIO_BUFFERS.get_recycled().await
    }

    pub async fn demux<R: Read + Seek>(
        &self,
        demuxer: &mut Demuxer<R>,
        chunk: Riff<Chunk>,
        mut buffer: AudioBuffer,
    ) -> Result<(), Error> {
        demuxer.read_chunk_data(chunk, &mut buffer.data)?;
        AUDIO_BUFFERS.send(buffer).await;
        Ok(())
    }

    pub async fn finish(self) {
        let mut buffer = AUDIO_BUFFERS.get_recycled().await;
        buffer.data.clear();
        AUDIO_BUFFERS.send(buffer).await;
    }
}

const fn audio_bytes_to_ms(bytes: usize) -> u32 {
    const BYTES_PER_MS: u32 =
        (SAMPLE_RATE * SAMPLE_CHANNELS as u32 * SAMPLE_DATA_FORMAT as u32 / 8) / 1000;
    bytes as u32 / BYTES_PER_MS
}
