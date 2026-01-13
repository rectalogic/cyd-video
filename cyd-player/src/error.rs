use core::fmt;
use display_interface::DisplayError;
use embedded_io::ReadExactError;
use embedded_sdmmc::SdCardError;
use esp_hal::spi::master::ConfigError;

#[derive(Debug)]
pub enum Error<IO, D>
where
    IO: fmt::Debug,
    D: fmt::Debug,
{
    SpiConfigError(ConfigError),
    SpiError(esp_hal::spi::Error),
    DisplayError(DisplayError),
    SdCardError(embedded_sdmmc::Error<SdCardError>),
    ReadError(IO),
    ReadExactError(ReadExactError<IO>),
    DecodeErrors(D),
    LoopEof,
}

impl<IO, D> From<ConfigError> for Error<IO, D>
where
    IO: fmt::Debug,
    D: fmt::Debug,
{
    fn from(value: ConfigError) -> Self {
        Error::SpiConfigError(value)
    }
}

impl<IO, D> From<DisplayError> for Error<IO, D>
where
    IO: fmt::Debug,
    D: fmt::Debug,
{
    fn from(value: DisplayError) -> Self {
        Error::DisplayError(value)
    }
}

impl<IO, D> From<embedded_sdmmc::Error<SdCardError>> for Error<IO, D>
where
    IO: fmt::Debug,
    D: fmt::Debug,
{
    fn from(value: embedded_sdmmc::Error<SdCardError>) -> Self {
        Error::SdCardError(value)
    }
}

impl<IO, D> From<ReadExactError<IO>> for Error<IO, D>
where
    IO: fmt::Debug,
    D: fmt::Debug,
{
    fn from(value: ReadExactError<IO>) -> Self {
        Error::ReadExactError(value)
    }
}

impl<IO, D> From<esp_hal::spi::Error> for Error<IO, D>
where
    IO: fmt::Debug,
    D: fmt::Debug,
{
    fn from(value: esp_hal::spi::Error) -> Self {
        Error::SpiError(value)
    }
}
