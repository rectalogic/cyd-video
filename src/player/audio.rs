use embassy_time::Delay;
use es8311::{ClockConfig, Es8311, Resolution};
use esp_hal::{
    dma_circular_buffers,
    gpio::{AnyPin, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, ConfigError as I2cConfigError, I2c},
    i2s::master::{
        Channels, Config as I2sConfig, ConfigError as I2sConfigError, DataFormat,
        Error as I2sError, I2s, asynch::I2sWriteDmaTransferAsync,
    },
    peripherals::{DMA_CH0, I2C0, I2S0},
    time::Rate,
};
use thiserror::Error;

pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_DATA_FORMAT: u16 = 16;
pub const SAMPLE_CHANNELS: u8 = 1;

const MCLK_FREQ: u32 = SAMPLE_RATE * 256; // 4_096_000 Hz
const DMA_SIZE: usize = 4096;

pub struct AudioPlayer {
    transaction: I2sWriteDmaTransferAsync<'static, &'static mut [u8; DMA_SIZE]>,
}

pub struct Peripherals {
    pub i2s: I2S0<'static>,
    pub i2c: I2C0<'static>,
    pub dma_channel: DMA_CH0<'static>,
    pub audio_enable: AnyPin<'static>,
    pub mclk: AnyPin<'static>,
    pub bclk: AnyPin<'static>,
    pub ws: AnyPin<'static>,
    pub dout: AnyPin<'static>,
    pub sda: AnyPin<'static>,
    pub scl: AnyPin<'static>,
}

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to configure audio I2S")]
    I2sConfig(#[from] I2sConfigError),
    #[error("I2S audio error")]
    I2s(I2sError),
    #[error("Failed to configure audio I2C")]
    I2cConfig(#[from] I2cConfigError),
    #[error("Failed to configure audio ES8311 codec")]
    Es8311(es8311::Error<esp_hal::i2c::master::Error>),
}

impl AudioPlayer {
    pub fn new(peripherals: Peripherals) -> Result<Self, AudioError> {
        let (_, _, tx_buffer, tx_descriptors) = dma_circular_buffers!(0, DMA_SIZE);

        let i2s_tx = {
            // Amp off initially (IO1 HIGH = disabled)
            let mut audio_enable = Output::new(
                peripherals.audio_enable,
                Level::High,
                OutputConfig::default(),
            );

            let i2s_tx = I2s::new(
                peripherals.i2s,
                peripherals.dma_channel,
                I2sConfig::new_tdm_philips()
                    .with_sample_rate(Rate::from_hz(SAMPLE_RATE))
                    .with_data_format(match SAMPLE_DATA_FORMAT {
                        16 => DataFormat::Data16Channel16,
                        32 => DataFormat::Data32Channel32,
                        _ => panic!("Invalid data format"),
                    })
                    .with_channels(match SAMPLE_CHANNELS {
                        1 => Channels::MONO,
                        2 => Channels::STEREO,
                        _ => panic!("Invalid channel count"),
                    }),
            )?
            .with_mclk(peripherals.mclk)
            .into_async()
            .i2s_tx
            .with_bclk(peripherals.bclk)
            .with_ws(peripherals.ws)
            .with_dout(peripherals.dout)
            .build(tx_descriptors);

            // Init ES8311 via I2C
            let mut i2c = I2c::new(
                peripherals.i2c,
                I2cConfig::default().with_frequency(Rate::from_khz(100)),
            )?
            .with_sda(peripherals.sda)
            .with_scl(peripherals.scl);

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
                .map_err(AudioError::Es8311)?;
            codec.volume_set(&mut i2c, 80, None).unwrap();
            codec.mute(&mut i2c, false).unwrap();

            // Enable amp
            audio_enable.set_low();

            i2s_tx
        };

        let transaction = i2s_tx
            .write_dma_circular_async(tx_buffer)
            .map_err(AudioError::I2s)?;

        Ok(Self { transaction })
    }

    pub async fn play(&mut self, mut frames: impl Iterator<Item = &[u8]>) {
        let Some(first_frame_pcm) = frames.next() else {
            return; // end of stream
        };

        let mut pending: &[u8] = first_frame_pcm;

        loop {
            if !pending.is_empty() {
                let written = self.transaction.push(pending).await.unwrap();
                pending = &pending[written..];
                continue;
            }

            let Some(next_frame_pcm) = frames.next() else {
                break; // end of stream
            };

            let written = self.transaction.push(next_frame_pcm).await.unwrap();
            if written < next_frame_pcm.len() {
                pending = &next_frame_pcm[written..];
            }
        }

        // End-of-stream: fill remaining buffer with silence to avoid looping
        const SILENCE: [u8; 512] = [0u8; 512];
        let mut silence_pushed: usize = 0;
        while silence_pushed < DMA_SIZE {
            let written = self.transaction.push(&SILENCE).await.unwrap();
            silence_pushed += written;
        }
    }
}

#[embassy_executor::task]
pub async fn audio_task(player: AudioPlayer) {}
