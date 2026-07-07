#![no_std]
#![no_main]

use embassy_executor::Spawner;
#[cfg(esp32s3)]
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
#[cfg(esp32s3)]
use {
    embassy_time::Delay,
    es8311::{ClockConfig, Es8311, Resolution},
    esp_hal::{
        gpio::{Level, Output, OutputConfig},
        i2c::master::{Config as I2cConfig, I2c},
    },
};

esp_bootloader_esp_idf::esp_app_desc!();

const SINE: [i16; 64] = [
    0, 3211, 6392, 9511, 12539, 15446, 18204, 20787, 23169, 25329, 27244, 28897, 30272, 31356,
    32137, 32609, 32767, 32609, 32137, 31356, 30272, 28897, 27244, 25329, 23169, 20787, 18204,
    15446, 12539, 9511, 6392, 3211, 0, -3211, -6392, -9511, -12539, -15446, -18204, -20787, -23169,
    -25329, -27244, -28897, -30272, -31356, -32137, -32609, -32767, -32609, -32137, -31356, -30272,
    -28897, -27244, -25329, -23169, -20787, -18204, -15446, -12539, -9511, -6392, -3211,
];

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    println!("Init!");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    #[cfg(esp32)]
    let dma_channel = peripherals.DMA_I2S0;
    #[cfg(esp32s3)]
    let dma_channel = peripherals.DMA_CH0;

    let (_, _, tx_buffer, tx_descriptors) = dma_buffers!(0, 32000);

    #[cfg(esp32)]
    let i2s_tx = {
        let i2s = I2s::new(
            peripherals.I2S0,
            dma_channel,
            Config::new_tdm_philips()
                .with_sample_rate(Rate::from_hz(44100))
                .with_data_format(DataFormat::Data16Channel16)
                .with_channels(Channels::STEREO),
        )
        .unwrap()
        .into_async();

        i2s.i2s_tx
            .with_bclk(peripherals.GPIO22)
            .with_ws(peripherals.GPIO27)
            .with_dout(peripherals.GPIO33)
            .build(tx_descriptors)
    };

    #[cfg(esp32s3)]
    let i2s_tx = {
        // Amp off initially (IO1 HIGH = disabled)
        let mut audio_enable = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());
        let mut delay = Delay;
        delay.delay_ms(10);

        // Build I2S TX
        let i2s_tx = I2s::new(
            peripherals.I2S0,
            dma_channel,
            Config::new_tdm_philips()
                .with_sample_rate(Rate::from_hz(44100))
                .with_data_format(DataFormat::Data16Channel16)
                .with_channels(Channels::STEREO),
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
                    mclk_frequency: 11_289_600,
                    sample_frequency: 44100,
                },
                Resolution::Bits16,
                Resolution::Bits16,
                &mut delay,
            )
            .unwrap();
        codec.volume_set(&mut i2c, 80, None).unwrap();
        codec.mute(&mut i2c, false).unwrap();
        println!("ES8311 initialized");

        // Let codec settle, then enable amp
        delay.delay_ms(100);
        audio_enable.set_low();
        println!("Amp enabled");

        i2s_tx
    };

    // Fill buffer with stereo-interleaved i16 samples (same value on both channels)
    let buffer = tx_buffer;
    let num_frames = buffer.len() / 4; // 4 bytes per stereo frame (2×i16)
    let mut sine_idx: usize = 0;
    for frame in 0..num_frames {
        let sample = SINE[sine_idx];
        let bytes = sample.to_le_bytes();
        let off = frame * 4;
        buffer[off + 0] = bytes[0];
        buffer[off + 1] = bytes[1];
        buffer[off + 2] = bytes[0];
        buffer[off + 3] = bytes[1];
        sine_idx += 1;
        if sine_idx >= SINE.len() {
            sine_idx = 0;
        }
    }

    let mut filler = [0u8; 10000];
    let filler_frames = filler.len() / 4;
    let mut sample_idx = num_frames % SINE.len();

    println!("Start");
    let mut transaction = i2s_tx.write_dma_circular_async(buffer).unwrap();
    loop {
        let mut si = sample_idx;
        for f in 0..filler_frames {
            let sample = SINE[si];
            let bytes = sample.to_le_bytes();
            let off = f * 4;
            filler[off + 0] = bytes[0];
            filler[off + 1] = bytes[1];
            filler[off + 2] = bytes[0];
            filler[off + 3] = bytes[1];
            si += 1;
            if si >= SINE.len() {
                si = 0;
            }
        }
        println!("Next");
        let written = transaction.push(&filler).await.unwrap();
        sample_idx = (sample_idx + written / 4) % SINE.len();
        println!("written {}", written);
    }
}
