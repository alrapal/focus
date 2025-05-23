#![no_std]
#![no_main]

use esp_hal::{
    clock::CpuClock, gpio::{Level, Output, OutputConfig}, main, spi::{
        master::{Config, Spi as BusSpi},
        Mode,
    }, time::{Duration, Instant, Rate},
    delay::Delay,
};
use esp_println::println;
use focus::spi_device_wrapper::SpiDeviceWrapper;
use gc9a01::{
    display::DisplayResolution240x240, prelude::DisplayRotation, Gc9a01, SPIDisplayInterface
};

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
    let mut delay = Delay::new();

    // BusSpi
    let sclk = peripherals.GPIO12;
    let miso = peripherals.GPIO11;
    let mosi = peripherals.GPIO13;
    // SpiDevice
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    // need to configure as output to respect bound trait for SPIDisplayInterface
    let dc = Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default());
    let mut rst = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    let mut spi: BusSpi<'_, esp_hal::Blocking> = BusSpi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(80))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_miso(miso)
    .with_mosi(mosi);

    let display_spi_device = SpiDeviceWrapper::new(&mut spi, cs, 4);

    let display_interface = SPIDisplayInterface::new(display_spi_device, dc);
    let mut display_driver = Gc9a01::new(
        display_interface,
        DisplayResolution240x240,
        DisplayRotation::Rotate0,
    ).into_buffered_graphics();
    display_driver.reset(&mut rst, &mut delay).unwrap();

    // let buffer: [u16; 240 * 240] = [0xff_u16; 240 * 240];
    display_driver.init_with_addr_mode(&mut delay).unwrap();

    let color_list :[u16; 4]= [
        0xF800_u16,
        0xFFE0_u16,
        0x7E0_u16,
        0x1F_u16
    ];
    let mut iter = color_list.iter().cycle();    

    loop {
        let delay_start = Instant::now();
        if let Some(color) = iter.next(){
            display_driver.fill(*color);
            display_driver.flush().unwrap();
        };
        println!("In Loop");
        
        while delay_start.elapsed() < Duration::from_millis(1000) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}
