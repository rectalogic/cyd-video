use crate::error::Error;
use core::{convert::Infallible, fmt};
use embedded_hal::spi::SpiBus;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{
    DirEntry, SdCardError, TimeSource, Timestamp, VolumeIdx, VolumeManager,
    filesystem::ToShortFileName,
};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    peripherals::{GPIO5, GPIO18, GPIO19, GPIO23, SPI3},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
};

pub struct Peripherals {
    pub spi3: SPI3<'static>,
    pub gpio5: GPIO5<'static>,
    pub gpio18: GPIO18<'static>,
    pub gpio19: GPIO19<'static>,
    pub gpio23: GPIO23<'static>,
}

type SdCardType =
    embedded_sdmmc::SdCard<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>, Delay>;
type VolumeManagerType = VolumeManager<SdCardType, DummyTimesource, 4, 4, 1>;
type FileType<'a> = embedded_sdmmc::File<'a, SdCardType, DummyTimesource, 4, 4, 1>;

pub struct SdCard {
    volume_manager: VolumeManagerType,
}

impl SdCard {
    pub fn new(
        peripherals: Peripherals,
    ) -> Result<Self, Error<Infallible, Infallible, Infallible>> {
        let mut spi = Spi::new(
            peripherals.spi3,
            SpiConfig::default().with_frequency(Rate::from_khz(400)), // <=400kHz required for initialization
        )?
        .with_sck(peripherals.gpio18)
        .with_mosi(peripherals.gpio23)
        .with_miso(peripherals.gpio19);

        // Send 74+ clock cycles (10 bytes = 80 cycles)
        // CS doesn't need to exist yet - it just needs to NOT be asserted
        let mut dummy = [0xFF; 10];
        SpiBus::transfer_in_place(&mut spi, &mut dummy)?;

        let cs = Output::new(peripherals.gpio5, Level::High, OutputConfig::default());
        let spi_dev = ExclusiveDevice::new(spi, cs, Delay::new()).unwrap();
        let sdcard = embedded_sdmmc::SdCard::new(spi_dev, Delay::new());

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

    pub fn iterate_dir<N, F>(
        &mut self,
        dirname: Option<N>,
        f: F,
    ) -> Result<(), Error<embedded_sdmmc::Error<SdCardError>, Infallible, Infallible>>
    where
        N: ToShortFileName,
        F: FnMut(&DirEntry),
    {
        let volume = self.volume_manager.open_volume(VolumeIdx(0))?;
        let directory = volume.open_root_dir()?;
        if let Some(dirname) = dirname {
            let subdir = directory.open_dir(dirname)?;
            subdir.iterate_dir(f)?;
            subdir.close()?;
        } else {
            directory.iterate_dir(f)?;
        }
        directory.close()?;
        volume.close()?;
        Ok(())
    }

    pub fn read_file<N, F, R, RE, DE>(
        &mut self,
        filename: N,
        f: F,
    ) -> Result<R, Error<embedded_sdmmc::Error<SdCardError>, RE, DE>>
    where
        RE: fmt::Debug,
        DE: fmt::Debug,
        N: ToShortFileName,
        F: FnOnce(&mut FileType) -> Result<R, Error<embedded_sdmmc::Error<SdCardError>, RE, DE>>,
    {
        let volume = self.volume_manager.open_volume(VolumeIdx(0))?;
        let directory = volume.open_root_dir()?;
        let mut file = directory.open_file_in_dir(filename, embedded_sdmmc::Mode::ReadOnly)?;

        let result = f(&mut file)?;

        // Close in reverse order
        file.close()?;
        directory.close()?;
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
