use core::fmt;
use display_interface::DisplayError;
use embedded_io::ReadExactError;
use embedded_sdmmc::SdCardError;
use esp_hal::spi::master::ConfigError;

#[derive(Debug)]
pub enum Error<D>
where
    D: fmt::Debug,
{
    SpiConfigError(ConfigError),
    DisplayError(DisplayError),
    SdCardError(embedded_sdmmc::Error<SdCardError>),
    ReadError(ReadExactError<D>),
}

impl<D> From<ConfigError> for Error<D>
where
    D: fmt::Debug,
{
    fn from(value: ConfigError) -> Self {
        Error::SpiConfigError(value)
    }
}

impl<D> From<DisplayError> for Error<D>
where
    D: fmt::Debug,
{
    fn from(value: DisplayError) -> Self {
        Error::DisplayError(value)
    }
}

impl<D> From<embedded_sdmmc::Error<SdCardError>> for Error<D>
where
    D: fmt::Debug,
{
    fn from(value: embedded_sdmmc::Error<SdCardError>) -> Self {
        Error::SdCardError(value)
    }
}

impl<D> From<ReadExactError<D>> for Error<D>
where
    D: fmt::Debug,
{
    fn from(value: ReadExactError<D>) -> Self {
        Error::ReadError(value)
    }
}
