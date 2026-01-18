#![deny(clippy::large_stack_frames)]

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
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
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::{ILI9341Rgb565, Model},
    options::{ColorOrder, Orientation, Rotation},
};

type InternalDisplay<'a> = mipidsi::Display<
    SpiInterface<'a, ExclusiveDevice<Spi<'a, Blocking>, Output<'a>, NoDelay>, Output<'a>>,
    ILI9341Rgb565,
    Output<'a>,
>;

pub struct Peripherals {
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
    display: InternalDisplay<'a>,
}

impl<'a> Display<'a> {
    pub fn new(display_buffer: &'a mut [u8], peripherals: Peripherals) -> Self {
        let spi = Spi::new(
            peripherals.spi2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(SpiMode::_0),
        )
        .expect("display SPI")
        //CLK
        .with_sck(peripherals.gpio14)
        //DIN
        .with_mosi(peripherals.gpio13)
        .with_miso(peripherals.gpio12);

        let dc = Output::new(peripherals.gpio2, Level::Low, OutputConfig::default());
        let cs = Output::new(peripherals.gpio15, Level::Low, OutputConfig::default());
        let mut reset = Output::new(peripherals.gpio4, Level::Low, OutputConfig::default());
        reset.set_high();

        let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).expect("infallible");
        let interface = SpiInterface::new(spi_dev, dc, display_buffer);

        let mut display = Builder::new(ILI9341Rgb565, interface)
            .reset_pin(reset)
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
            .init(&mut Delay::new())
            .expect("display builder init");

        let _backlight = Output::new(peripherals.gpio21, Level::High, OutputConfig::default());
        display.clear(Rgb565::BLACK).expect("display clear");

        Self { display }
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

        let delay = Delay::new();
        loop {
            delay.delay_millis(5000);
        }
    }
}

impl<'a> Deref for Display<'a> {
    type Target = InternalDisplay<'a>;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl<'a> DerefMut for Display<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.display
    }
}
