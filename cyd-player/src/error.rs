use core::fmt;
use embedded_io::ReadExactError;
use embedded_sdmmc::SdCardError;
use esp_hal::spi::master::ConfigError;

#[derive(Debug)]
pub enum Error<IO, D, DI>
where
    IO: fmt::Debug,
    D: fmt::Debug,
    DI: fmt::Debug,
{
    SpiConfigError(ConfigError),
    SpiError(esp_hal::spi::Error),
    DisplayError(DI),
    SdCardError(embedded_sdmmc::Error<SdCardError>),
    ReadError(IO),
    ReadExactError(ReadExactError<IO>),
    DecodeErrors(D),
    VideoEof,
}

impl<IO, D, DI> From<ConfigError> for Error<IO, D, DI>
where
    IO: fmt::Debug,
    D: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: ConfigError) -> Self {
        Error::SpiConfigError(value)
    }
}

impl<IO, D, DI> From<embedded_sdmmc::Error<SdCardError>> for Error<IO, D, DI>
where
    IO: fmt::Debug,
    D: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: embedded_sdmmc::Error<SdCardError>) -> Self {
        Error::SdCardError(value)
    }
}

impl<IO, D, DI> From<ReadExactError<IO>> for Error<IO, D, DI>
where
    IO: fmt::Debug,
    D: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: ReadExactError<IO>) -> Self {
        Error::ReadExactError(value)
    }
}

impl<IO, D, DI> From<esp_hal::spi::Error> for Error<IO, D, DI>
where
    IO: fmt::Debug,
    D: fmt::Debug,
    DI: fmt::Debug,
{
    fn from(value: esp_hal::spi::Error) -> Self {
        Error::SpiError(value)
    }
}
