#![no_std]
#![no_main]

use embedded_hal::spi::{Error, ErrorType, Operation, SpiBus, SpiDevice};
use esp_hal::{
    clock::CpuClock,
    gpio::{Output, OutputConfig},
    main,
    spi::{
        master::{Config, Spi},
        Mode,
    },
    time::{Duration, Instant, Rate},
};
use esp_println::println;
use gc9a01::SPIDisplayInterface;

struct SpiDeviceWrapper<'a, SPI> {
    spi: &'a mut SPI,
}

impl<'a, SPI, E> ErrorType for SpiDeviceWrapper<'a, SPI>
where
    SPI: SpiBus<u8, Error = E>,
    E: Error,
{
    type Error = E;
}

impl<'a, SPI, E> SpiDevice for SpiDeviceWrapper<'a, SPI>
where
    SPI: embedded_hal::spi::SpiBus<u8, Error = E>,
    E: embedded_hal::spi::Error,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        for operation in operations {
            //! Todo: Implement the rest of the transaction logic
            match operation {
                Operation::Write(data) => {
                    self.spi.write(data)?;
                }
                Operation::Transfer(write, read) => {
                    self.spi.transfer(write, read)?;
                }
                Operation::TransferInPlace(words) => {
                    self.spi.transfer_in_place(words)?;
                }
                Operation::Read(data) => {
                    self.spi.read(data)?;
                }
                Operation::DelayNs(delay) => {
                    // At 240Mhz, 1 tick is more or less 4ns. (4.16)
                    const NS_PER_TICK: u32 = 4;
                    let mut ns_in_tick = *delay / NS_PER_TICK;

                    while ns_in_tick > 0 {
                        ns_in_tick -= 1;
                    }
                }
            }
        }
        Ok(())
    }
}

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", e);
    loop {}
}

#[main]
fn main() -> ! {
    // generator version: 0.3.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let sclk = peripherals.GPIO12;
    let miso = peripherals.GPIO11;
    let mosi = peripherals.GPIO13;
    let cs = peripherals.GPIO10;
    // need to configure as output to respect bound trait for SPIDisplayInterface
    let dc = Output::new(
        peripherals.GPIO3,
        esp_hal::gpio::Level::Low,
        OutputConfig::default(),
    );

    let mut spi: Spi<'_, esp_hal::Blocking> = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_miso(miso)
    .with_mosi(mosi)
    .with_cs(cs); // unstable

    let mut display_spi_device_wrapper = SpiDeviceWrapper { spi: &mut spi };

    let display_interface = SPIDisplayInterface::new(display_spi_device_wrapper, dc);

    loop {
        let delay_start = Instant::now();

        while delay_start.elapsed() < Duration::from_millis(250) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}
