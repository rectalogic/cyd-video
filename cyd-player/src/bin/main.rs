#![cfg_attr(not(test), no_std)]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::ops::DerefMut;
use cyd_player::error::Error;
#[cfg(feature = "mjpeg")]
use cyd_player::video::mjpeg;
#[cfg(feature = "rgb")]
use cyd_player::video::rgb;
#[cfg(feature = "yuv")]
use cyd_player::video::yuv;

use embedded_sdmmc::SdCardError;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;

#[cfg(feature = "alloc")]
extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_hal::main]
fn main() -> ! {
    // generator version: 1.1.0

    #[cfg(feature = "log")]
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    #[cfg(feature = "alloc")]
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let mut display = cyd_player::display::Display::new(cyd_player::display::Peripherals {
        spi2: peripherals.SPI2,
        gpio2: peripherals.GPIO2,
        gpio4: peripherals.GPIO4,
        gpio12: peripherals.GPIO12,
        gpio13: peripherals.GPIO13,
        gpio14: peripherals.GPIO14,
        gpio15: peripherals.GPIO15,
        gpio21: peripherals.GPIO21,
    })
    .unwrap();
    let mut sdcard = match cyd_player::sdcard::SdCard::new(cyd_player::sdcard::Peripherals {
        spi3: peripherals.SPI3,
        gpio5: peripherals.GPIO5,
        gpio18: peripherals.GPIO18,
        gpio19: peripherals.GPIO19,
        gpio23: peripherals.GPIO23,
    }) {
        Ok(sdcard) => sdcard,
        Err(e) => display.message(format_args!("SD card error: {e:?}")),
    };

    #[cfg(feature = "mjpeg")]
    if let Err(e) = sdcard.read_file(
        "video.mjp",
        |file| -> Result<(), Error<embedded_sdmmc::Error<SdCardError>, _>> {
            cyd_player::video::play::<_, _, _, _, { mjpeg::DECODE_SIZE }, mjpeg::MjpegDecoder<_>>(
                file,
                display.deref_mut(),
            )
        },
    ) {
        display.message(format_args!("{e:?}"));
    }

    #[cfg(feature = "yuv")]
    if let Err(e) = sdcard.read_file(
        "video.yuv",
        |file| -> Result<(), Error<embedded_sdmmc::Error<SdCardError>, _>> {
            cyd_player::video::play::<_, _, _, _, { yuv::DECODE_SIZE }, yuv::YuvDecoder<_>>(
                file,
                display.deref_mut(),
            )
        },
    ) {
        display.message(format_args!("{e:?}"));
    }

    #[cfg(feature = "rgb")]
    if let Err(e) = sdcard.read_file(
        "video.rgb",
        |file| -> Result<(), Error<embedded_sdmmc::Error<SdCardError>, _>> {
            cyd_player::video::play::<_, _, _, _, { rgb::DECODE_SIZE }, rgb::RgbDecoder<_>>(
                file,
                display.deref_mut(),
            )
        },
    ) {
        display.message(format_args!("{e:?}"));
    }

    unreachable!();
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
