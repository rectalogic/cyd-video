use crate::error::Error;
use core::{convert::Infallible, ops::AsyncFnOnce};
use embassy_time::Delay;
use embedded_hal::spi::SpiBus;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{
    SdCardError, TimeSource, Timestamp, VolumeIdx, VolumeManager, filesystem::ToShortFileName,
};
#[cfg(esp32)]
use esp_hal::peripherals::{GPIO5, GPIO18, GPIO19, GPIO23, SPI3};
#[cfg(esp32s3)]
use esp_hal::peripherals::{GPIO38, GPIO39, GPIO40, GPIO47, SPI3};
use esp_hal::{
    Blocking,
    gpio::{Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
};

pub struct Peripherals {
    pub spi3: SPI3<'static>,
    #[cfg(esp32)]
    pub cs: GPIO5<'static>,
    #[cfg(esp32s3)]
    pub cs: GPIO47<'static>,
    #[cfg(esp32)]
    pub sclk: GPIO18<'static>,
    #[cfg(esp32s3)]
    pub sclk: GPIO38<'static>,
    #[cfg(esp32)]
    pub miso: GPIO19<'static>,
    #[cfg(esp32s3)]
    pub miso: GPIO39<'static>,
    #[cfg(esp32)]
    pub mosi: GPIO23<'static>,
    #[cfg(esp32s3)]
    pub mosi: GPIO40<'static>,
}

type SdCardType =
    embedded_sdmmc::SdCard<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>, Delay>;
type VolumeManagerType = VolumeManager<SdCardType, DummyTimesource, 4, 4, 1>;
pub type DirectoryType<'a> = embedded_sdmmc::Directory<'a, SdCardType, DummyTimesource, 4, 4, 1>;
pub struct SdCard {
    volume_manager: VolumeManagerType,
}

impl SdCard {
    pub fn new(peripherals: Peripherals) -> Result<Self, Error<Infallible, Infallible>> {
        let mut spi = Spi::new(
            peripherals.spi3,
            SpiConfig::default().with_frequency(Rate::from_khz(400)), // <=400kHz required for initialization
        )?
        .with_sck(peripherals.sclk)
        .with_mosi(peripherals.mosi)
        .with_miso(peripherals.miso);

        // Send 74+ clock cycles (10 bytes = 80 cycles)
        // CS doesn't need to exist yet - it just needs to NOT be asserted
        let mut dummy = [0xFF; 10];
        SpiBus::transfer_in_place(&mut spi, &mut dummy)?;

        let cs = Output::new(peripherals.cs, Level::High, OutputConfig::default());
        let spi_dev = ExclusiveDevice::new(spi, cs, Delay).unwrap();
        let sdcard = embedded_sdmmc::SdCard::new(spi_dev, Delay);

        // Force initialization
        let _ = sdcard.num_bytes();

        // Reconfigure frequency
        sdcard.spi(|spi| {
            spi.bus_mut()
                .apply_config(&SpiConfig::default().with_frequency(Rate::from_mhz(80)))
        })?;

        let volume_manager = VolumeManager::new(sdcard, DummyTimesource);

        Ok(Self { volume_manager })
    }

    pub async fn open_directory<DN, F, R>(
        &mut self,
        dirname: DN,
        f: F,
    ) -> Result<R, Error<embedded_sdmmc::Error<SdCardError>, Infallible>>
    where
        DN: ToShortFileName,
        for<'a> F: AsyncFnOnce(
            &'a DirectoryType<'a>,
        )
            -> Result<R, Error<embedded_sdmmc::Error<SdCardError>, Infallible>>,
    {
        let volume = self.volume_manager.open_volume(VolumeIdx(0))?;
        let root_directory = volume.open_root_dir()?;
        let directory = root_directory.open_dir(dirname)?;

        let result = f(&directory).await?;

        // Close in reverse order
        directory.close()?;
        root_directory.close()?;
        volume.close()?;

        Ok(result)
    }
}

pub struct DummyTimesource;

impl TimeSource for DummyTimesource {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}
