use super::{SAMPLE_CHANNELS, SAMPLE_DATA_FORMAT, SAMPLE_RATE};
use embassy_time::Delay;
use es8311::{ClockConfig, Es8311, Resolution};
use esp_hal::{
    Async, dma_circular_buffers,
    gpio::{AnyPin, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, ConfigError as I2cConfigError, I2c},
    i2s::master::{
        Channels, Config as I2sConfig, ConfigError as I2sConfigError, DataFormat,
        Error as I2sError, I2s, I2sTx, asynch::I2sWriteDmaTransferAsync,
    },
    peripherals::{DMA_CH0, I2C0, I2S0},
    time::Rate,
};
use thiserror::Error;

const MCLK_FREQ: u32 = SAMPLE_RATE * 256; // 4_096_000 Hz

pub struct AudioDevice {
    i2s_state: Option<I2sState>,
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

enum I2sState {
    I2sTx {
        i2s_tx: I2sTx<'static, Async>,
        tx_buffer: &'static mut [u8; AudioDevice::DMA_SIZE],
    },
    I2sWriteDmaTransferAsync(
        I2sWriteDmaTransferAsync<'static, &'static mut [u8; AudioDevice::DMA_SIZE]>,
    ),
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

impl AudioDevice {
    pub const DMA_SIZE: usize = 4096;

    pub fn new(peripherals: Peripherals) -> Result<Self, AudioError> {
        let (_, _, tx_buffer, tx_descriptors) = dma_circular_buffers!(0, AudioDevice::DMA_SIZE);

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

        Ok(Self {
            i2s_state: Some(I2sState::I2sTx { i2s_tx, tx_buffer }),
        })
    }

    pub async fn available(&mut self) -> Result<usize, AudioError> {
        match self.i2s_state {
            Some(I2sState::I2sWriteDmaTransferAsync(ref mut transaction)) => {
                transaction.available().await.map_err(AudioError::I2s)
            }
            _ => Ok(AudioDevice::DMA_SIZE),
        }
    }

    pub async fn push(&mut self, buffer: &[u8]) -> Result<usize, AudioError> {
        if let Some(I2sState::I2sWriteDmaTransferAsync(ref mut transaction)) = self.i2s_state {
            return transaction.push(buffer).await.map_err(AudioError::I2s);
        }

        let i2s_state = self.i2s_state.take().unwrap();
        match i2s_state {
            I2sState::I2sTx { i2s_tx, tx_buffer } => {
                let mut transaction = i2s_tx
                    .write_dma_circular_async(tx_buffer)
                    .map_err(AudioError::I2s)?;
                let result = transaction.push(buffer).await.map_err(AudioError::I2s);
                self.i2s_state = Some(I2sState::I2sWriteDmaTransferAsync(transaction));
                result
            }
            _ => unreachable!(),
        }
    }
}
