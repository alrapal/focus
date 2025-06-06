use esp_hal::{
    gpio::GpioPin,
    peripherals::SPI2,
    spi::master::{Config, Spi},
    time::Rate,
    Blocking,
};

pub fn init_spi_bus(
    spi_peripheral: SPI2,
    sclk: GpioPin<12>,
    mosi: GpioPin<13>,
) -> Spi<'static, Blocking> {
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
