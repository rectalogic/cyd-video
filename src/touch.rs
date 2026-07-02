use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};
use critical_section::Mutex;
#[cfg(esp32s3)]
use esp_hal::peripherals::GPIO17;
#[cfg(esp32)]
use esp_hal::peripherals::GPIO36;
use esp_hal::{
    gpio::{Event, Input, InputConfig, Io, Pull},
    handler,
    peripherals::IO_MUX,
    ram,
};

static TOUCH: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static TOUCHED: AtomicBool = AtomicBool::new(false);

pub struct Peripherals {
    #[cfg(esp32)]
    pub irq: GPIO36<'static>,
    #[cfg(esp32s3)]
    pub irq: GPIO17<'static>,
}

/// Detect XPT2046 touches
pub struct TouchDetector(Io<'static>);

impl TouchDetector {
    pub fn new(io_mux: IO_MUX<'static>, peripherals: Peripherals) -> Self {
        let mut io = Io::new(io_mux);
        io.set_interrupt_handler(touch_handler);

        let mut touch = Input::new(peripherals.irq, InputConfig::default().with_pull(Pull::Up));

        critical_section::with(|cs| {
            touch.listen(Event::FallingEdge);
            TOUCH.borrow_ref_mut(cs).replace(touch)
        });

        Self(io)
    }

    pub fn was_touched(&self) -> bool {
        TOUCHED.swap(false, Ordering::Relaxed)
    }
}

#[handler]
#[ram]
fn touch_handler() {
    if critical_section::with(|cs| {
        TOUCH
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .is_interrupt_set()
    }) {
        TOUCHED.store(true, Ordering::Relaxed);
    }

    critical_section::with(|cs| TOUCH.borrow_ref_mut(cs).as_mut().unwrap().clear_interrupt());
}
