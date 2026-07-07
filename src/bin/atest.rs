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
                .with_data_format(DataFormat::Data32Channel16)
                .with_channels(Channels::STEREO),
        )
        .unwrap()
        .with_mclk(peripherals.GPIO4)
        .into_async()
        .i2s_tx
        .with_bclk(peripherals.GPIO5)
        .with_ws(peripherals.GPIO7)
        .with_dout(peripherals.GPIO6)
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

    let data =
        unsafe { core::slice::from_raw_parts(&SINE as *const _ as *const u8, SINE.len() * 2) };

    let buffer = tx_buffer;
    let mut idx = 0;
    for i in 0..usize::min(data.len(), buffer.len()) {
        buffer[i] = data[idx];
        idx += 1;
        if idx >= data.len() {
            idx = 0;
        }
    }

    let mut filler = [0u8; 10000];
    let mut idx = 32000 % data.len();

    println!("Start");
    let mut transaction = i2s_tx.write_dma_circular_async(buffer).unwrap();
    loop {
        for i in 0..filler.len() {
            filler[i] = data[(idx + i) % data.len()];
        }
        println!("Next");
        let written = transaction.push(&filler).await.unwrap();
        idx = (idx + written) % data.len();
        println!("written {}", written);
    }
}
