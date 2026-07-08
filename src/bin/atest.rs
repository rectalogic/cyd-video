#![no_std]
#![no_main]
#![cfg(esp32s3)]

use embassy_executor::Spawner;
use embedded_hal::delay::DelayNs;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    dma_buffers,
    i2s::master::{Channels, Config, DataFormat, I2s},
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

/// Copy mono s16le samples from `pcm` into `buf`, wrapping at EOF.
/// Returns the number of bytes written.
fn fill_buf(buf: &mut [u8], pcm: &[u8], pos: &mut usize) {
    let mut written = 0;
    while written < buf.len() {
        let chunk = buf.len() - written;
        let remaining = pcm.len() - *pos;
        if chunk <= remaining {
            buf[written..written + chunk].copy_from_slice(&pcm[*pos..*pos + chunk]);
            written += chunk;
            *pos += chunk;
            if *pos >= pcm.len() {
                *pos = 0;
            }
        } else {
            buf[written..written + remaining].copy_from_slice(&pcm[*pos..*pos + remaining]);
            written += remaining;
            *pos = 0;
        }
    }
}

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

    let (_, _, tx_buffer, tx_descriptors) = dma_buffers!(0, 32000);

    let i2s_tx = {
        // Amp off initially (IO1 HIGH = disabled)
        let mut audio_enable = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());
        let mut delay = Delay;
        delay.delay_ms(10);

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

    // Fill initial DMA buffer from PCM
    let buffer = tx_buffer;
    let mut pcm_pos: usize = 0;
    fill_buf(buffer, PCM, &mut pcm_pos);

    let mut filler = [0u8; 10000];

    println!("Start");
    let mut transaction = i2s_tx.write_dma_circular_async(buffer).unwrap();
    loop {
        fill_buf(&mut filler, PCM, &mut pcm_pos);
        let written = transaction.push(&filler).await.unwrap();
        // If the circular buffer couldn't accept all data, rewind pcm_pos
        if written < filler.len() {
            pcm_pos = (pcm_pos + PCM.len() - (filler.len() - written)) % PCM.len();
        }
        println!("pos {}/{}", pcm_pos, PCM.len());
    }
}
