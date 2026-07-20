use crate::{
    display::DisplayAsyncMutex,
    error::Error,
    player::{
        buffers::{Buffer, Buffers},
        demux::Demuxer,
    },
};
use core::{
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};
pub use device::{AudioDevice, AudioError, Peripherals};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};
use embedded_io::{Read, Seek};
use riffparse::{Chunk, Riff};

mod device;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

static AUDIO_BUFFERS: Buffers<3, Buffer<3>> = Buffers::new();

static AUDIO_CLOCK: AtomicU32 = AtomicU32::new(0);
static AUDIO_CLOCK_STARTED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

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
    const TIMEOUT: Duration =
        Duration::from_millis(AudioClock::audio_bytes_to_ms(SILENCE.len()) as u64 / 2);
    audio_device.push(&SILENCE).await?;

    while AUDIO_BUFFERS.receive_timeout(TIMEOUT).await.is_err() {
        audio_device.push(&SILENCE).await?;
    }
    Ok(())
}

async fn play(audio_device: &mut AudioDevice) -> Result<(), Error> {
    let mut audio_clock: Option<AudioClock> = None;

    loop {
        let buffer = AUDIO_BUFFERS.receive().await;
        if buffer.data.is_empty() {
            break; // End-of-stream signaled by empty buffer
        }
        let mut remaining = buffer.data.as_slice();
        while !remaining.is_empty() {
            match audio_clock {
                Some(ref clock) => {
                    remaining = &remaining[audio_device.push(remaining).await?..];
                    clock.elapsed();
                }
                None => {
                    let old_audio_bytes = AudioDevice::DMA_SIZE - audio_device.available().await?;
                    let latency_ms = AudioClock::audio_bytes_to_ms(old_audio_bytes);
                    remaining = &remaining[audio_device.push(remaining).await?..];
                    audio_clock = Some(AudioClock::start(Instant::now(), latency_ms));
                }
            };
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

    pub async fn demux<R: Read + Seek>(
        &self,
        demuxer: &mut Demuxer<R>,
        chunk: Riff<Chunk>,
    ) -> Result<(), Error> {
        let mut buffer = AUDIO_BUFFERS.get_recycled().await;
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

pub struct AudioClock {
    time: Instant,
    latency_ms: u32,
}

impl AudioClock {
    fn start(time: Instant, latency_ms: u32) -> Self {
        Self::store(0);
        AUDIO_CLOCK_STARTED.signal(());
        Self { time, latency_ms }
    }

    pub async fn started() {
        AUDIO_CLOCK_STARTED.wait().await;
    }

    fn elapsed(&self) -> u32 {
        Self::store((self.time.elapsed().as_millis() as u32).saturating_sub(self.latency_ms))
    }

    fn store(ms: u32) -> u32 {
        AUDIO_CLOCK.store(ms, Ordering::Relaxed);
        ms
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
