use embedded_io::ReadExactError;
use embedded_sdmmc::SdCardError;
use esp_hal::spi::master::ConfigError;
use riffparse::binrw;

use crate::{display::DisplayError, video::mjpeg::MjpegError};

#[derive(Debug)]
pub enum Error {
    SpiConfigError(ConfigError),
    SpiError(esp_hal::spi::Error),
    DisplayError(DisplayError),
    SdCardError(embedded_sdmmc::Error<SdCardError>),
    ReadError(embedded_sdmmc::Error<SdCardError>),
    ReadExactError(ReadExactError<embedded_sdmmc::Error<SdCardError>>),
    BinReadError(binrw::Error),
    DecodeErrors(MjpegError),
}

impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Error::SpiConfigError(value)
    }
}

impl From<embedded_sdmmc::Error<SdCardError>> for Error {
    fn from(value: embedded_sdmmc::Error<SdCardError>) -> Self {
        Error::SdCardError(value)
    }
}

impl From<ReadExactError<embedded_sdmmc::Error<SdCardError>>> for Error {
    fn from(value: ReadExactError<embedded_sdmmc::Error<SdCardError>>) -> Self {
        Error::ReadExactError(value)
    }
}

impl From<esp_hal::spi::Error> for Error {
    fn from(value: esp_hal::spi::Error) -> Self {
        Error::SpiError(value)
    }
}

impl From<binrw::Error> for Error {
    fn from(value: binrw::Error) -> Self {
        Error::BinReadError(value)
    }
}

impl From<MjpegError> for Error {
    fn from(value: MjpegError) -> Self {
        Error::DecodeErrors(value)
    }
}
