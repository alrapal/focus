#![no_std]
#![no_main]

use core::cell::RefCell;
use critical_section::Mutex;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, RgbColor, WebColors},
    primitives::{Circle, PrimitiveStyle},
    Drawable,
};
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
use focus::
    hardware::{button, spi_bus, screen}
;

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", e);
    loop {}
}

static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static SPI_BUS: Mutex<RefCell<Option<Spi<'static, Blocking>>>> = Mutex::new(RefCell::new(None));

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
    // let _cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    // need to configure as output to respect bound trait for SPIDisplayInterface
    // let dc = Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default());
    let mut rst = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    let spi = spi_bus::init_spi_bus(peripherals.SPI2, peripherals.GPIO12, peripherals.GPIO13);

    critical_section::with(|cs| {
        SPI_BUS.borrow_ref_mut(cs).replace(spi);
    });

    let mut display_driver = screen::init_screen(peripherals.GPIO10, peripherals.GPIO3, &SPI_BUS);

    display_driver.reset(&mut rst, &mut delay).unwrap();

    display_driver.init_with_addr_mode(&mut delay).unwrap();
    display_driver.fill(0);

    let mut decrease = false;
    let mut radius = 50_u32;
    let center = Point::new(120, 120);
    let mut top_left = Point::new(center.x - radius as i32, center.y - radius as i32);
    let circle_style = PrimitiveStyle::with_fill(Rgb565::RED);
    let mut circle = Circle::new(top_left, radius * 2).into_styled(circle_style);

    circle.draw(&mut display_driver).unwrap();
    display_driver.flush().unwrap();

    let color_list: [Rgb565; 4] = [
        Rgb565::CSS_ALICE_BLUE,
        Rgb565::CSS_YELLOW,
        Rgb565::CSS_SEA_GREEN,
        Rgb565::CSS_SALMON,
    ];
    let mut iter = color_list.iter().cycle();

    loop {
        let delay_start = Instant::now();

        if radius == 0 {
            if let Some(color) = iter.next() {
                circle.style.fill_color = Some(*color);
            }
            decrease = false;
        } else if radius == center.y as u32 {
            decrease = true;
        }

        if decrease {
            display_driver.fill(0);
            radius -= 1;
        } else {
            radius += 1;
        }

        top_left = Point::new(center.x - radius as i32, center.y - radius as i32);
        circle.primitive.diameter = radius * 2;
        circle.primitive.top_left = top_left;
        circle.draw(&mut display_driver).unwrap();
        display_driver.flush().unwrap();
        // if let Some(color) = iter.next() {
        //     display_driver.fill(*color);
        //     display_driver.flush().unwrap();
        // };
        // println!("In Loop");

        while delay_start.elapsed() < Duration::from_millis(20) {}
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
