#![cfg_attr(not(test), no_std)]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use cyd_player::touch::TouchDetector;
use embassy_executor::Spawner;

use esp_backtrace as _;
use esp_hal::{clock::CpuClock, timer::timg::TimerGroup};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    // generator version: 1.1.0

    const AVI_DIRECTORY: &str = "AVI";

    #[cfg(feature = "log")]
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    #[cfg(esp32)]
    const SIZE: usize = 98768;
    #[cfg(esp32s3)]
    const SIZE: usize = 73744;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: SIZE);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    #[cfg(esp32)]
    let display_peripherals = cyd_player::display::Peripherals {
        spi2: peripherals.SPI2,
        dc: peripherals.GPIO2.into(),
        rst: Some(peripherals.GPIO4.into()),
        mosi: peripherals.GPIO13.into(),
        sclk: peripherals.GPIO14.into(),
        cs: peripherals.GPIO15.into(),
        bl: peripherals.GPIO21.into(),
    };
    #[cfg(esp32s3)]
    let display_peripherals = cyd_player::display::Peripherals {
        spi2: peripherals.SPI2,
        dc: peripherals.GPIO46.into(),
        rst: None,
        mosi: peripherals.GPIO11.into(),
        sclk: peripherals.GPIO12.into(),
        cs: peripherals.GPIO10.into(),
        bl: peripherals.GPIO45.into(),
    };
    let mut display = cyd_player::display::Display::new(display_peripherals);
    #[cfg(esp32)]
    let sdcard_peripherals = cyd_player::sdcard::Peripherals {
        spi3: peripherals.SPI3,
        cs: peripherals.GPIO5.into(),
        sclk: peripherals.GPIO18.into(),
        miso: peripherals.GPIO19.into(),
        mosi: peripherals.GPIO23.into(),
    };
    #[cfg(esp32s3)]
    let sdcard_peripherals = cyd_player::sdcard::Peripherals {
        spi3: peripherals.SPI3,
        cs: peripherals.GPIO47.into(),
        sclk: peripherals.GPIO38.into(),
        miso: peripherals.GPIO39.into(),
        mosi: peripherals.GPIO40.into(),
    };
    let mut sdcard = match cyd_player::sdcard::SdCard::new(sdcard_peripherals) {
        Ok(sdcard) => sdcard,
        Err(e) => display.message(format_args!("SD card error: {e:?}")),
    };

    #[cfg(esp32)]
    let touch_peripherals = cyd_player::touch::Peripherals {
        irq: peripherals.GPIO36.into(),
    };
    #[cfg(esp32s3)]
    let touch_peripherals = cyd_player::touch::Peripherals {
        irq: peripherals.GPIO17.into(),
    };
    let touch_detector = TouchDetector::new(peripherals.IO_MUX, touch_peripherals);

    log::info!("Loading dir {AVI_DIRECTORY}");
    if let Err(e) =
        cyd_player::video::play_directory(AVI_DIRECTORY, &mut sdcard, &mut display, &touch_detector)
            .await
    {
        display.message(format_args!("{e:?}"))
    };

    unreachable!();
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
