use core::fmt;
use embedded_io::ReadExactError;
use embedded_sdmmc::SdCardError;
use esp_hal::spi::master::ConfigError;
use riffparse::binrw;

use crate::video::decoder::MjpegError;

#[derive(Debug)]
pub enum Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    SpiConfigError(ConfigError),
    SpiError(esp_hal::spi::Error),
    DisplayError(DI),
    SdCardError(embedded_sdmmc::Error<SdCardError>),
    ReadError(IO),
    ReadExactError(ReadExactError<IO>),
    BinReadError(binrw::Error),
    DecodeErrors(MjpegError),
}

impl<IO, DI> From<ConfigError> for Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: ConfigError) -> Self {
        Error::SpiConfigError(value)
    }
}

impl<IO, DI> From<embedded_sdmmc::Error<SdCardError>> for Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: embedded_sdmmc::Error<SdCardError>) -> Self {
        Error::SdCardError(value)
    }
}

impl<IO, DI> From<ReadExactError<IO>> for Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: ReadExactError<IO>) -> Self {
        Error::ReadExactError(value)
    }
}

impl<IO, DI> From<esp_hal::spi::Error> for Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: esp_hal::spi::Error) -> Self {
        Error::SpiError(value)
    }
}

impl<IO, DI> From<binrw::Error> for Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: binrw::Error) -> Self {
        Error::BinReadError(value)
    }
}

impl<IO, DI> From<MjpegError> for Error<IO, DI>
where
    IO: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: MjpegError) -> Self {
        Error::DecodeErrors(value)
    }
}
