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
use embedded_hal::digital::PinState;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
    spi::master::Spi,
    time::Instant,
    Blocking,
};
use esp_println::println;
use focus::hardware::{screen, spi_bus};
use hl_driver::{
    debounce,
    switch::{self, Pressable},
};

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", e);
    loop {}
}

// static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static SPI_BUS: Mutex<RefCell<Option<Spi<'static, Blocking>>>> = Mutex::new(RefCell::new(None));

#[main]
fn main() -> ! {
    // generator version: 0.3.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let mut delay = Delay::new();

    // Switches
    let mut boot_button = switch::Switch::new(
        Input::new(
            peripherals.GPIO0,
            InputConfig::default().with_pull(Pull::Up),
        ),
        PinState::Low,
    )
    .with_debounce(debounce::Debouncer::default());

    let mut hy040_switch = switch::Switch::new(
        Input::new(
            peripherals.GPIO6,
            InputConfig::default().with_pull(esp_hal::gpio::Pull::Up),
        ),
        PinState::Low,
    )
    .with_debounce(debounce::Debouncer::default());

    // SPI Bus
    let spi = spi_bus::init_spi_bus(peripherals.SPI2, peripherals.GPIO12, peripherals.GPIO13);
    critical_section::with(|cs| {
        SPI_BUS.borrow_ref_mut(cs).replace(spi);
    });

    // Screen
    let mut display_driver = screen::init_screen(peripherals.GPIO10, peripherals.GPIO3, &SPI_BUS);
    let mut rst = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    display_driver.reset(&mut rst, &mut delay).unwrap();

    display_driver.init_with_addr_mode(&mut delay).unwrap();
    display_driver.fill(0);

    // Shape
    let mut decrease = false;
    let mut radius = 50_u32;
    let center = Point::new(120, 120);
    let mut top_left = Point::new(center.x - radius as i32, center.y - radius as i32);
    let circle_style = PrimitiveStyle::with_fill(Rgb565::RED);
    let mut circle = Circle::new(top_left, radius * 2).into_styled(circle_style);

    circle.draw(&mut display_driver).unwrap();
    display_driver.flush().unwrap();

    const COLOR_LIST: [Rgb565; 4] = [
        Rgb565::CSS_DARK_RED,
        Rgb565::CSS_AQUA,
        Rgb565::CSS_YELLOW_GREEN,
        Rgb565::CSS_BLUE_VIOLET,
    ];

    let mut iter = COLOR_LIST.iter().cycle();

    loop {
        let _delay_start = Instant::now();

        if boot_button.has_been_pressed().unwrap() {
            println!("Reset radius");
            radius = 0;
            display_driver.fill(0);
        }

        if hy040_switch.has_been_pressed().unwrap() {
            if let Some(color) = iter.next() {
                println!("Change color");
                circle.style.fill_color = Some(*color);
            }
        }

        if radius == 0 {
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

        // while delay_start.elapsed() < Duration::from_millis(200) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}
