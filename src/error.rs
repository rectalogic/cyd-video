use embedded_io::ReadExactError;
use embedded_sdmmc::SdCardError;
use esp_hal::spi::master::ConfigError;
use riffparse::binrw;

use crate::{
    display::DisplayError,
    player::{DemuxError, audio::AudioError, video::MjpegError},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("SPI configuration failed: `{0}`")]
    SpiConfig(#[from] ConfigError),
    #[error("SPI error: `{0:?}`")]
    Spi(esp_hal::spi::Error),
    #[error("Display error: `{0:?}`")]
    Display(DisplayError),
    #[error("SD card error: `{0}`")]
    SdCard(#[from] SdCardError),
    #[error("SD card error: `{0}`")]
    SdCardError(#[from] embedded_sdmmc::Error<SdCardError>),
    #[error("SD card read exact error: `{0}`")]
    ReadExact(#[from] ReadExactError<embedded_sdmmc::Error<SdCardError>>),
    #[error("bin read error: `{0:?}`")]
    BinRead(binrw::Error),
    #[error("MJPEG decode error: `{0}`")]
    Decode(#[from] MjpegError),
    #[error("AVI demux error: `{0}`")]
    Demux(#[from] DemuxError),
    #[error("AVI audio error: `{0}`")]
    Audio(#[from] AudioError),
}

impl From<esp_hal::spi::Error> for Error {
    fn from(value: esp_hal::spi::Error) -> Self {
        Error::Spi(value)
    }
}

impl From<binrw::Error> for Error {
    fn from(value: binrw::Error) -> Self {
        Error::BinRead(value)
    }
}
