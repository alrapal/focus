mod spi_peripheral;

// Crate re-export
pub use gc9a01::{
    display::DisplayResolution240x240, prelude::DisplayRotation, Gc9a01, SPIDisplayInterface,
};

pub use spi_peripheral::{SpiPeripheral, SpiPeripheralError};
