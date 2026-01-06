#![cfg_attr(not(test), no_std)]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embedded_hal::delay::DelayNs;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay};
use log::info;

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

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut delay = Delay::new();

    let mut display = cyd_video::Display::new(cyd_video::DisplayPeripherals {
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

    display.draw();

    loop {
        info!("Hello world!");
        delay.delay_ms(1000u32);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
