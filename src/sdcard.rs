use crate::error::Error;
use core::{convert::Infallible, ops::AsyncFnOnce};
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{
    SdCardError, TimeSource, Timestamp, VolumeIdx, VolumeManager, filesystem::ToShortFileName,
};
use esp_hal::{
    Blocking,
    gpio::{AnyPin, Level, Output, OutputConfig},
    peripherals::SPI3,
    spi::{
        Mode as SpiMode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};

pub struct Peripherals {
    pub spi3: SPI3<'static>,
    pub cs: AnyPin<'static>,
    pub sclk: AnyPin<'static>,
    pub miso: AnyPin<'static>,
    pub mosi: AnyPin<'static>,
}

type SdCardType =
    embedded_sdmmc::SdCard<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>, Delay>;
type VolumeManagerType = VolumeManager<SdCardType, DummyTimesource, 4, 4, 1>;
pub type DirectoryType<'a> = embedded_sdmmc::Directory<'a, SdCardType, DummyTimesource, 4, 4, 1>;

pub struct SdCard {
    volume_manager: VolumeManagerType,
}

impl SdCard {
    pub fn new(peripherals: Peripherals) -> Result<Self, Error<Infallible>> {
        let cs = Output::new(peripherals.cs, Level::High, OutputConfig::default());
        let spi = Spi::new(
            peripherals.spi3,
            SpiConfig::default()
                .with_frequency(Rate::from_khz(400)) // <=400kHz required for initialization
                .with_mode(SpiMode::_0),
        )?
        .with_sck(peripherals.sclk)
        .with_mosi(peripherals.mosi)
        .with_miso(peripherals.miso);

        let spi_dev = ExclusiveDevice::new(spi, cs, Delay).unwrap();
        let sdcard = embedded_sdmmc::SdCard::new(spi_dev, Delay);

        // Force initialization
        let _ = sdcard.num_bytes();

        // Reclock
        sdcard.spi(|spi| {
            spi.bus_mut().apply_config(
                &SpiConfig::default()
                    .with_frequency(Rate::from_mhz(40))
                    .with_mode(SpiMode::_0),
            )
        })?;

        let volume_manager = VolumeManager::new(sdcard, DummyTimesource);

        Ok(Self { volume_manager })
    }

    pub async fn open_directory<DN, F, R>(
        &mut self,
        dirname: DN,
        f: F,
    ) -> Result<R, Error<embedded_sdmmc::Error<SdCardError>>>
    where
        DN: ToShortFileName,
        for<'a> F: AsyncFnOnce(
            &'a DirectoryType<'a>,
        ) -> Result<R, Error<embedded_sdmmc::Error<SdCardError>>>,
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
