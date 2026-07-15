#![deny(clippy::large_stack_frames)]

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex};
use embassy_time::Delay;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::{
    delay::DelayNs,
    digital::{ErrorType, OutputPin},
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{
    gpio::{AnyPin, Level, Output, OutputConfig},
    peripherals::SPI2,
    spi::{
        master::{Config as SpiConfig, Spi},
        Mode as SpiMode,
    },
    time::Rate,
    Blocking,
};
use mipidsi::{
    interface::SpiInterface,
    models::{ILI9341Rgb565, Model},
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
    Builder, NoResetPin,
};
use static_cell::StaticCell;

type DisplayTypeRst<RST> = mipidsi::Display<
    SpiInterface<'static, ExclusiveDevice<Spi<'static, Blocking>, NoCs, NoDelay>, Output<'static>>,
    ILI9341Rgb565,
    RST,
>;
type DisplayType = DisplayTypeRst<NoResetPin>;

pub type DisplayAsyncMutex = mutex::Mutex<CriticalSectionRawMutex, Display>;

pub type DisplayError = <DisplayType as DrawTarget>::Error;

pub const DISPLAY_WIDTH: u32 = ILI9341Rgb565::FRAMEBUFFER_SIZE.0 as u32;
pub const DISPLAY_HEIGHT: u32 = ILI9341Rgb565::FRAMEBUFFER_SIZE.1 as u32;
pub const CENTER: Point = Point::new(
    (ILI9341Rgb565::FRAMEBUFFER_SIZE.1 / 2) as i32,
    (ILI9341Rgb565::FRAMEBUFFER_SIZE.0 / 2) as i32,
);

pub struct Peripherals {
    pub spi2: SPI2<'static>,
    pub dc: AnyPin<'static>,
    // miso GPIO11 not needed
    pub mosi: AnyPin<'static>,
    pub sclk: AnyPin<'static>,
    pub cs: AnyPin<'static>,
    pub bl: AnyPin<'static>,
}

pub struct Display {
    display: DisplayType,
}

impl Display {
    #[expect(clippy::large_stack_frames)]
    pub fn new(peripherals: Peripherals) -> &'static DisplayAsyncMutex {
        let spi = Spi::new(
            peripherals.spi2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(SpiMode::_0),
        )
        .expect("display SPI")
        .with_sck(peripherals.sclk)
        .with_mosi(peripherals.mosi)
        .with_cs(peripherals.cs);

        let dc = Output::new(peripherals.dc, Level::Low, OutputConfig::default());

        static STATIC_CELL: StaticCell<[u8; 512]> = StaticCell::new();
        let display_buffer = STATIC_CELL.init([0_u8; 512]);

        let spi_dev = ExclusiveDevice::new_no_delay(spi, NoCs).expect("infallible");
        let interface = SpiInterface::new(spi_dev, dc, display_buffer);

        let mut display = Builder::new(ILI9341Rgb565, interface)
            .invert_colors(ColorInversion::Inverted)
            .display_size(
                ILI9341Rgb565::FRAMEBUFFER_SIZE.0,
                ILI9341Rgb565::FRAMEBUFFER_SIZE.1,
            )
            .color_order(ColorOrder::Bgr)
            .orientation(
                Orientation::new()
                    .rotate(Rotation::Deg270)
                    .flip_horizontal(),
            )
            .init(&mut Delay)
            .expect("display builder init");

        let _backlight = Output::new(peripherals.bl, Level::High, OutputConfig::default());
        display.clear(Rgb565::BLACK).expect("display clear");

        static DISPLAY: StaticCell<DisplayAsyncMutex> = StaticCell::new();
        DISPLAY.init(mutex::Mutex::new(Self { display }))
    }

    pub fn message(&mut self, args: fmt::Arguments) -> ! {
        let mut buf = [0u8; 256];
        let message = format_no_std::show(&mut buf, args).unwrap();
        log::error!("{message}");
        self.display.clear(Rgb565::BLACK).unwrap();
        let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
        Text::with_baseline(message, Point::default(), style, Baseline::Top)
            .draw(&mut self.display)
            .unwrap();

        let mut delay = Delay;
        loop {
            delay.delay_ms(5000);
        }
    }
}

impl Deref for Display {
    type Target = DisplayType;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl DerefMut for Display {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.display
    }
}

pub struct NoCs;

impl OutputPin for NoCs {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ErrorType for NoCs {
    type Error = core::convert::Infallible;
}
