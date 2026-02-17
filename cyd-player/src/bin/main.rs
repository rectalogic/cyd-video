#![cfg_attr(not(test), no_std)]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::ops::DerefMut;

use cyd_player::touch::TouchDetector;
use embassy_executor::Spawner;
use embedded_sdmmc::ShortFileName;
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

    #[cfg(feature = "log")]
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    let mut display_buffer = [0u8; 512];
    let mut display = cyd_player::display::Display::new(
        &mut display_buffer,
        cyd_player::display::Peripherals {
            spi2: peripherals.SPI2,
            dc: peripherals.GPIO2,
            rst: peripherals.GPIO4,
            mosi: peripherals.GPIO13,
            sclk: peripherals.GPIO14,
            cs: peripherals.GPIO15,
            bl: peripherals.GPIO21,
        },
    );
    let mut sdcard = match cyd_player::sdcard::SdCard::new(cyd_player::sdcard::Peripherals {
        spi3: peripherals.SPI3,
        cs: peripherals.GPIO5,
        sclk: peripherals.GPIO18,
        miso: peripherals.GPIO19,
        mosi: peripherals.GPIO23,
    }) {
        Ok(sdcard) => sdcard,
        Err(e) => display.message(format_args!("SD card error: {e:?}")),
    };

    let touch_detector = TouchDetector::new(peripherals.IO_MUX, peripherals.GPIO36);

    let avi_dir = env!("AVI_DIRECTORY");
    log::info!("Loading dir {avi_dir}");
    if let Err(e) = sdcard
        .open_directory(avi_dir, async |directory| {
            const MAX_FILES: usize = 5;
            let mut filenames: [Option<ShortFileName>; MAX_FILES] = [None; _];
            let mut index: usize = 0;
            if let Err(e) = directory.iterate_dir(|entry| {
                if index < MAX_FILES
                    && !entry.attributes.is_directory()
                    && entry.name.extension() == b"AVI"
                {
                    log::info!("Found {}", entry.name);
                    filenames[index] = Some(entry.name);
                    index += 1;
                };
            }) {
                display.message(format_args!("directory {avi_dir} error: {e:?}"))
            };
            filenames.sort();

            let filenames_cycle = filenames.into_iter().flatten().cycle();
            for filename in filenames_cycle {
                log::info!("Playing {filename}");
                match directory.open_file_in_dir(filename, embedded_sdmmc::Mode::ReadOnly) {
                    Ok(file) => {
                        match cyd_player::video::play(file, display.deref_mut(), &touch_detector)
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => display.message(format_args!("{e:?}")),
                        }
                    }
                    Err(e) => display.message(format_args!("{filename} error: {e:?}")),
                };
            }

            Ok(())
        })
        .await
    {
        display.message(format_args!("{e:?}"))
    };

    unreachable!();
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
