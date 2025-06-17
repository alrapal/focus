use esp_hal::{
    peripherals::{GPIO12, GPIO13, SPI2},
    spi::master::{Config, Spi},
    time::Rate,
    Blocking,
};

pub fn init_spi_bus<'l>(
    spi_peripheral: SPI2<'l>,
    sclk: GPIO12<'l>,
    mosi: GPIO13<'l>,
) -> Spi<'l, Blocking> {
    Spi::new(
        spi_peripheral,
        Config::default()
            .with_frequency(Rate::from_mhz(80))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
}
