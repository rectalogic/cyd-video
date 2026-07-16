#![cfg_attr(not(test), no_std)]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

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
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.1.0

    const AVI_DIRECTORY: &str = "AVI";

    #[cfg(feature = "log")]
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let display_peripherals = cyd_player::display::Peripherals {
        spi2: peripherals.SPI2,
        dc: peripherals.GPIO46.into(),
        mosi: peripherals.GPIO11.into(),
        sclk: peripherals.GPIO12.into(),
        cs: peripherals.GPIO10.into(),
        bl: peripherals.GPIO45.into(),
    };
    let display = cyd_player::display::Display::new(display_peripherals);

    let sdcard_peripherals = cyd_player::sdcard::Peripherals {
        spi3: peripherals.SPI3,
        cs: peripherals.GPIO47.into(),
        sclk: peripherals.GPIO38.into(),
        miso: peripherals.GPIO39.into(),
        mosi: peripherals.GPIO40.into(),
    };
    let mut sdcard = match cyd_player::sdcard::SdCard::new(sdcard_peripherals) {
        Ok(sdcard) => sdcard,
        Err(e) => display
            .lock()
            .await
            .message(format_args!("SD card error: {e:?}")),
    };

    let touch_peripherals = cyd_player::touch::Peripherals {
        irq: peripherals.GPIO17.into(),
    };
    let touch_detector =
        cyd_player::touch::TouchDetector::new(peripherals.IO_MUX, touch_peripherals);

    let audio_peripherals = cyd_player::player::audio::Peripherals {
        i2s: peripherals.I2S0,
        i2c: peripherals.I2C0,
        dma_channel: peripherals.DMA_CH0,
        audio_enable: peripherals.GPIO1.into(),
        mclk: peripherals.GPIO4.into(),
        bclk: peripherals.GPIO5.into(),
        ws: peripherals.GPIO7.into(),
        dout: peripherals.GPIO8.into(),
        sda: peripherals.GPIO16.into(),
        scl: peripherals.GPIO15.into(),
    };
    let audio_spawn_token = match cyd_player::player::audio::audio_task(audio_peripherals, display)
    {
        Ok(spawn_token) => spawn_token,
        Err(e) => display
            .lock()
            .await
            .message(format_args!("Failed to spawn audio task: {e:?}")),
    };
    spawner.spawn(audio_spawn_token);

    let video_spawn_token = match cyd_player::player::video::video_task(display) {
        Ok(spawn_token) => spawn_token,
        Err(e) => display
            .lock()
            .await
            .message(format_args!("Failed to spawn video task: {e:?}")),
    };
    spawner.spawn(video_spawn_token);

    log::info!("Loading dir {AVI_DIRECTORY}");
    if let Err(e) =
        cyd_player::player::play_directory(AVI_DIRECTORY, &mut sdcard, display, &touch_detector)
            .await
    {
        display.lock().await.message(format_args!("{e:?}"))
    };

    unreachable!();
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
