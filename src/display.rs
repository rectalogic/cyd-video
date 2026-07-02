#![deny(clippy::large_stack_frames)]

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use embassy_time::Delay;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::{
    delay::DelayNs,
    digital::{ErrorType, OutputPin},
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
#[cfg(esp32)]
use esp_hal::peripherals::{GPIO2, GPIO4, GPIO13, GPIO14, GPIO15, GPIO21, SPI2};
#[cfg(esp32s3)]
use esp_hal::peripherals::{GPIO10, GPIO11, GPIO12, GPIO45, GPIO46, SPI2};
use esp_hal::{
    Blocking,
    gpio::{Level, Output, OutputConfig},
    spi::{
        Mode as SpiMode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
#[cfg(esp32s3)]
use mipidsi::NoResetPin;
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::{ILI9341Rgb565, Model},
    options::{ColorOrder, Orientation, Rotation},
};

type DisplayTypeRst<'a, RST> = mipidsi::Display<
    SpiInterface<'a, ExclusiveDevice<Spi<'a, Blocking>, NoCs, NoDelay>, Output<'a>>,
    ILI9341Rgb565,
    RST,
>;
#[cfg(esp32)]
type DisplayType<'a> = DisplayTypeRst<'a, Output<'a>>;
#[cfg(esp32s3)]
type DisplayType<'a> = DisplayTypeRst<'a, NoResetPin>;

pub const DISPLAY_WIDTH: u32 = ILI9341Rgb565::FRAMEBUFFER_SIZE.0 as u32;
pub const DISPLAY_HEIGHT: u32 = ILI9341Rgb565::FRAMEBUFFER_SIZE.1 as u32;
pub const CENTER: Point = Point::new(
    (ILI9341Rgb565::FRAMEBUFFER_SIZE.1 / 2) as i32,
    (ILI9341Rgb565::FRAMEBUFFER_SIZE.0 / 2) as i32,
);

pub struct Peripherals {
    pub spi2: SPI2<'static>,
    #[cfg(esp32)]
    pub dc: GPIO2<'static>,
    #[cfg(esp32s3)]
    pub dc: GPIO46<'static>,
    #[cfg(esp32)]
    pub rst: GPIO4<'static>,
    // No RST for esp32s3
    // miso GPIO12(esp32)/GPIO11(esp32s3) not needed
    #[cfg(esp32)]
    pub mosi: GPIO13<'static>,
    #[cfg(esp32s3)]
    pub mosi: GPIO11<'static>,
    #[cfg(esp32)]
    pub sclk: GPIO14<'static>,
    #[cfg(esp32s3)]
    pub sclk: GPIO12<'static>,
    #[cfg(esp32)]
    pub cs: GPIO15<'static>,
    #[cfg(esp32s3)]
    pub cs: GPIO10<'static>,
    #[cfg(esp32)]
    pub bl: GPIO21<'static>,
    #[cfg(esp32s3)]
    pub bl: GPIO45<'static>,
}

pub struct Display<'a> {
    display: DisplayType<'a>,
}

impl<'a> Display<'a> {
    #[allow(clippy::large_stack_frames)]
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
        .with_cs(peripherals.cs);

        let dc = Output::new(peripherals.dc, Level::Low, OutputConfig::default());

        let spi_dev = ExclusiveDevice::new_no_delay(spi, NoCs).expect("infallible");
        let interface = SpiInterface::new(spi_dev, dc, display_buffer);

        #[cfg(esp32)]
        let mut display_builder = {
            let mut rst = Output::new(peripherals.rst, Level::Low, OutputConfig::default());
            rst.set_high();
            Builder::new(ILI9341Rgb565, interface).reset_pin(rst)
        };
        #[cfg(esp32s3)]
        let mut display_builder = Builder::new(ILI9341Rgb565, interface);

        display_builder = display_builder
            .display_size(
                ILI9341Rgb565::FRAMEBUFFER_SIZE.0,
                ILI9341Rgb565::FRAMEBUFFER_SIZE.1,
            )
            .color_order(ColorOrder::Bgr)
            .orientation(
                Orientation::new()
                    .rotate(Rotation::Deg270)
                    .flip_horizontal(),
            );

        let mut display = display_builder
            .init(&mut Delay)
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

        let mut delay = Delay;
        loop {
            delay.delay_ms(5000);
        }
    }
}

impl<'a> Deref for Display<'a> {
    type Target = DisplayType<'a>;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl<'a> DerefMut for Display<'a> {
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
