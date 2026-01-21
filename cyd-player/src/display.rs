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

pub const CENTER: Point = Point::new(
    (ILI9341Rgb565::FRAMEBUFFER_SIZE.1 / 2) as i32,
    (ILI9341Rgb565::FRAMEBUFFER_SIZE.0 / 2) as i32,
);

pub struct Peripherals {
    pub spi2: SPI2<'static>,
    pub dc: GPIO2<'static>,
    pub rst: GPIO4<'static>,
    pub miso: GPIO12<'static>,
    pub mosi: GPIO13<'static>,
    pub sclk: GPIO14<'static>,
    pub cs: GPIO15<'static>,
    pub bl: GPIO21<'static>,
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
        .with_sck(peripherals.sclk)
        .with_mosi(peripherals.mosi)
        .with_miso(peripherals.miso);

        let dc = Output::new(peripherals.dc, Level::Low, OutputConfig::default());
        let cs = Output::new(peripherals.cs, Level::Low, OutputConfig::default());
        let mut rst = Output::new(peripherals.rst, Level::Low, OutputConfig::default());
        rst.set_high();

        let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).expect("infallible");
        let interface = SpiInterface::new(spi_dev, dc, display_buffer);

        let mut display = Builder::new(ILI9341Rgb565, interface)
            .reset_pin(rst)
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

        let _backlight = Output::new(peripherals.bl, Level::High, OutputConfig::default());
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
