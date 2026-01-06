#![deny(clippy::large_stack_frames)]

use display_interface_spi::SPIInterface;

use embedded_graphics::{
    draw_target::DrawTarget, pixelcolor::Rgb565, prelude::*, primitives::Rectangle,
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    peripherals::{GPIO2, GPIO4, GPIO12, GPIO13, GPIO14, GPIO15, GPIO21, SPI2},
    spi::{
        Mode as SpiMode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use ili9341::{DisplaySize240x320, Ili9341, Orientation};

type InternalDisplay<'a> = Ili9341<
    SPIInterface<ExclusiveDevice<Spi<'a, Blocking>, Output<'a>, NoDelay>, Output<'a>>,
    Output<'a>,
>;

pub struct DisplayPeripherals {
    pub spi2: SPI2<'static>,
    pub gpio2: GPIO2<'static>,
    pub gpio4: GPIO4<'static>,
    pub gpio12: GPIO12<'static>,
    pub gpio13: GPIO13<'static>,
    pub gpio14: GPIO14<'static>,
    pub gpio15: GPIO15<'static>,
    pub gpio21: GPIO21<'static>,
}

pub struct Display<'a> {
    _backlight: Output<'a>,
    display: InternalDisplay<'a>,
}

impl<'a> Display<'a> {
    pub fn new(peripherals: DisplayPeripherals) -> Self {
        let spi = Spi::new(
            peripherals.spi2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(SpiMode::_0),
        )
        .unwrap()
        //CLK
        .with_sck(peripherals.gpio14)
        //DIN
        .with_mosi(peripherals.gpio13)
        .with_miso(peripherals.gpio12);

        let dc = Output::new(peripherals.gpio2, Level::Low, OutputConfig::default());
        let cs = Output::new(peripherals.gpio15, Level::Low, OutputConfig::default());
        let reset = Output::new(peripherals.gpio4, Level::Low, OutputConfig::default());

        let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
        let interface = SPIInterface::new(spi_dev, dc);

        let display = Ili9341::new(
            interface,
            reset,
            &mut Delay::new(),
            Orientation::Landscape,
            DisplaySize240x320,
        )
        .unwrap();

        let _backlight = Output::new(peripherals.gpio21, Level::High, OutputConfig::default());

        Self {
            _backlight,
            display,
        }
    }

    pub fn draw(&mut self) {
        self.display.clear(Rgb565::BLUE).unwrap();
        self.display
            .fill_solid(
                &Rectangle::new(Point::new(30, 20), Size::new(50, 50)),
                Rgb565::RED,
            )
            .unwrap();
    }
}
