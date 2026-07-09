#![no_std]
#![no_main]
#![cfg(esp32s3)]

use core::iter::repeat_n;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    dma_circular_buffers,
    i2s::master::{Channels, Config, DataFormat, I2s, asynch::I2sWriteDmaTransferAsync},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::println;
use {
    embassy_time::Delay,
    es8311::{ClockConfig, Es8311, Resolution},
    esp_hal::{
        gpio::{Level, Output, OutputConfig},
        i2c::master::{Config as I2cConfig, I2c},
    },
};

esp_bootloader_esp_idf::esp_app_desc!();

/// PCM audio encoded as: `ffmpeg … -acodec pcm_s16le -ar 16000 -ac 1`
/// Signed 16-bit little-endian, 16 kHz, mono.
static PCM: &[u8] = include_bytes!("audio.pcm");

const SAMPLE_RATE: u32 = 16_000;
const MCLK_FREQ: u32 = SAMPLE_RATE * 256; // 4_096_000 Hz
const DMA_SIZE: usize = 4096;

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    println!("Init!");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let dma_channel = peripherals.DMA_CH0;

    // 4096 bytes = 128 ms at 16 kHz 16-bit mono — enough DMA headroom
    // without noticeable drain delay at end-of-stream.
    let (_, _, tx_buffer, tx_descriptors) = dma_circular_buffers!(0, DMA_SIZE);

    let i2s_tx = {
        // Amp off initially (IO1 HIGH = disabled)
        let mut audio_enable = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());

        // Build I2S TX — mono: hardware duplicates each sample to both L/R channels
        let i2s_tx = I2s::new(
            peripherals.I2S0,
            dma_channel,
            Config::new_tdm_philips()
                .with_sample_rate(Rate::from_hz(SAMPLE_RATE))
                .with_data_format(DataFormat::Data16Channel16)
                .with_channels(Channels::MONO),
        )
        .unwrap()
        .with_mclk(peripherals.GPIO4)
        .into_async()
        .i2s_tx
        .with_bclk(peripherals.GPIO5)
        .with_ws(peripherals.GPIO7)
        .with_dout(peripherals.GPIO8)
        .build(tx_descriptors);

        // Init ES8311 via I2C
        let mut i2c = I2c::new(
            peripherals.I2C0,
            I2cConfig::default().with_frequency(Rate::from_khz(100)),
        )
        .unwrap()
        .with_sda(peripherals.GPIO16)
        .with_scl(peripherals.GPIO15);

        let mut delay = Delay;
        let codec = Es8311::new(0x18);
        codec
            .init(
                &mut i2c,
                &ClockConfig {
                    mclk_inverted: false,
                    sclk_inverted: false,
                    mclk_from_mclk_pin: true,
                    mclk_frequency: MCLK_FREQ,
                    sample_frequency: SAMPLE_RATE,
                },
                Resolution::Bits16,
                Resolution::Bits16,
                &mut delay,
            )
            .unwrap();
        codec.volume_set(&mut i2c, 80, None).unwrap();
        codec.mute(&mut i2c, false).unwrap();
        println!("ES8311 initialized");

        // Enable amp
        audio_enable.set_low();
        println!("Amp enabled");

        i2s_tx
    };

    println!(
        "PCM bytes: {} ({:.1} s)",
        PCM.len(),
        PCM.len() as f32 / (SAMPLE_RATE * 2) as f32
    );

    // Don't pre-load the buffer. TxCircularState::update() in esp-hal uses
    // `ptr < descr_address` (strict less-than) to compute available bytes,
    // so the first completed descriptor is never credited. With any pre-load,
    // push() always lags behind the DMA by one descriptor, causing the DMA
    // to wrap onto stale/unfilled descriptors before push catches up.
    // Starting empty means push() feeds the first descriptor via the normal
    // available → write channel, producing a predictable ~42-84ms of initial
    // silence (1-2 descriptor completion times) instead of the unpredictable
    // ~250ms skip caused by the tracking lag.
    //
    // The delay before playback should be DMA_SIZE / (SAMPLE_RATE × bytes_per_sample) millis
    let mut transaction = i2s_tx.write_dma_circular_async(tx_buffer).unwrap();

    play(&mut transaction, repeat_n(PCM, 4)).await;
}

async fn play(
    transaction: &mut I2sWriteDmaTransferAsync<'_, &mut [u8; DMA_SIZE]>,
    mut frames: impl Iterator<Item = &[u8]>,
) {
    let Some(first_frame_pcm) = frames.next() else {
        return; // end of stream
    };

    let mut pending: &[u8] = first_frame_pcm;

    loop {
        if !pending.is_empty() {
            let written = transaction.push(pending).await.unwrap();
            pending = &pending[written..];
            continue;
        }

        let Some(next_frame_pcm) = frames.next() else {
            break; // end of stream
        };

        let written = transaction.push(next_frame_pcm).await.unwrap();
        if written < next_frame_pcm.len() {
            pending = &next_frame_pcm[written..];
        }
    }

    // End-of-stream: fill remaining buffer with silence to avoid looping
    // Option A: push silence explicitly
    const SILENCE: [u8; 512] = [0u8; 512];
    let mut silence_pushed: usize = 0;
    while silence_pushed < DMA_SIZE {
        let written = transaction.push(&SILENCE).await.unwrap();
        silence_pushed += written;
    }

    println!("Done");
}
