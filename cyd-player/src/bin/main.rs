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
use embedded_sdmmc::SdCardError;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;

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
    if let Err(e) = sdcard.read_file(
        "video.cyd",
        |file| -> Result<(), Error<embedded_sdmmc::Error<SdCardError>>> {
            match cyd_player::video::play(file, display.deref_mut()) {
                Err(e) => display.message(format_args!("{e:?}")),
                Ok(_) => unreachable!(),
            }
        },
    ) {
        display.message(format_args!("Load video failed: {e:?}"));
    }

    unreachable!();
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
