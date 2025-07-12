use core::cell::RefCell;

use crate::drivers::SpiPeripheral;
use critical_section::Mutex;
use esp_hal::spi::Error;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    peripherals::{GPIO10, GPIO3},
    spi::master::Spi,
    Blocking,
};
use gc9a01::{
    mode::BufferedGraphics,
    prelude::{DisplayResolution240x240, DisplayRotation, SPIInterface},
    Gc9a01, SPIDisplayInterface,
};

// Complex type for the SPI interface
type DisplaySpiInterface = SPIInterface<
    SpiPeripheral<'static, Spi<'static, Blocking>, Error, Output<'static>>,
    Output<'static>,
>;

// Complex type for the Screen driver
pub type DisplayDriver = Gc9a01<
    DisplaySpiInterface,
    DisplayResolution240x240,
    BufferedGraphics<DisplayResolution240x240>,
>;

pub fn init_screen(
    cs: GPIO10<'static>,
    dc: GPIO3<'static>,
    mutex_bus: &'static Mutex<RefCell<Option<Spi<'static, Blocking>>>>,
) -> DisplayDriver {
    // Configure the pins as ouputs
    let cs = Output::new(cs, esp_hal::gpio::Level::High, OutputConfig::default());
    let dc = Output::new(dc, Level::Low, OutputConfig::default());
    // Spi peripheral wrapper for usage within the SPI display interface (Gc9a1 library requirement, works with SpiDevice trait).
    let spi_peripheral = SpiPeripheral::new(mutex_bus, cs, 4);
    // Spi interface used by the screen driver
    let interface = SPIDisplayInterface::new(spi_peripheral, dc);
    // Screen driver. Given as buffered_graphics to be used with embedded_graphics library
    Gc9a01::new(
        interface,
        DisplayResolution240x240,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics()
}
