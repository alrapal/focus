#![no_std]
#![no_main]

use core::cell::RefCell;
use critical_section::Mutex;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, Io, Level, Output, OutputConfig},
    handler, main, ram,
    spi::master::Spi,
    time::{Duration, Instant},
    Blocking,
};
use esp_println::println;
use focus::{
    drivers::{
        DisplayResolution240x240, DisplayRotation, Gc9a01, SPIDisplayInterface, SpiPeripheral,
    },
    hardware::{button, spi_bus},
};

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", e);
    loop {}
}

static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static SPI_BUS: Mutex<RefCell<Option<Spi<'_, Blocking>>>> = Mutex::new(RefCell::new(None));

#[main]
fn main() -> ! {
    // generator version: 0.3.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let mut delay = Delay::new();

    //IO mulitplexer for gpio interrupt
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(button_handler);

    // Button
    let mut boot_button = button::init_boot_button(peripherals.GPIO0);

    critical_section::with(|cs| {
        boot_button.listen(esp_hal::gpio::Event::FallingEdge);
        BUTTON.borrow_ref_mut(cs).replace(boot_button);
    });

    // SpiDevice
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    // need to configure as output to respect bound trait for SPIDisplayInterface
    let dc = Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default());
    let mut rst = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    let spi = spi_bus::init_spi_bus(peripherals.SPI2, peripherals.GPIO12, peripherals.GPIO13);

    critical_section::with(|cs| {
        SPI_BUS.borrow_ref_mut(cs).replace(spi);
    });

    let display_spi_device = SpiPeripheral::new(&SPI_BUS, cs, 4);

    let display_interface = SPIDisplayInterface::new(display_spi_device, dc);
    let mut display_driver = Gc9a01::new(
        display_interface,
        DisplayResolution240x240,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics();

    display_driver.reset(&mut rst, &mut delay).unwrap();

    display_driver.init_with_addr_mode(&mut delay).unwrap();

    let color_list: [u16; 4] = [0xF800_u16, 0xFFE0_u16, 0x7E0_u16, 0x1F_u16];
    let mut iter = color_list.iter().cycle();

    loop {
        let delay_start = Instant::now();
        if let Some(color) = iter.next() {
            display_driver.fill(*color);
            display_driver.flush().unwrap();
        };
        println!("In Loop");

        while delay_start.elapsed() < Duration::from_millis(1000) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}

#[handler]
#[ram]
fn button_handler() {
    println!("GPIO interrupt");

    if critical_section::with(|cs| {
        BUTTON
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .is_interrupt_set()
    }) {
        println!("Button was source of interrupt");
    } else {
        println!("Button was not source of interrupt");
    }

    critical_section::with(|cs| {
        BUTTON
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt();
    })
}
