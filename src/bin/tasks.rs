//! embassy hello world
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup};

type CancelSignal = Signal<CriticalSectionRawMutex, ()>;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run(signal: &'static CancelSignal) {
    while signal.try_take().is_none() {
        esp_println::println!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
    esp_println::println!("Task finished");
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    static SIGNAL: CancelSignal = CancelSignal::new();

    spawner.spawn(run(&SIGNAL).unwrap());
    Timer::after(Duration::from_millis(3_000)).await;
    esp_println::println!("Signal first task");
    SIGNAL.signal(());
    spawner.spawn(run(&SIGNAL).unwrap());
    esp_println::println!("Signal second task");
    SIGNAL.signal(());

    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
