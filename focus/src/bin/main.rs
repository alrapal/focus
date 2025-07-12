#![no_std]
#![no_main]

use core::{cell::RefCell, iter::Cycle, slice::Iter, sync::atomic::AtomicI32};
use critical_section::Mutex;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, WebColors},
    primitives::{Circle, PrimitiveStyle, Styled},
    Drawable,
};
use embedded_hal::digital::PinState;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    handler, main,
    peripherals::{GPIO0, GPIO4, GPIO5, GPIO6},
    ram,
    spi::master::Spi,
    time::Duration,
    timer::{self, timg::TimerGroup, PeriodicTimer},
    Blocking,
};
use esp_println::println;
use focus::hardware::{
    screen::{self, DisplayDriver},
    spi_bus,
};
use hl_driver::{
    debounce,
    encoder::{self, Direction, Encode, Hy040},
    switch::{self, DebouncedSwitch, Pressable},
};

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", e);
    loop {}
}

// List color to change to change the background color
const COLOR_LIST: [Rgb565; 4] = [
    Rgb565::CSS_RED,
    Rgb565::CSS_GREEN,
    Rgb565::CSS_YELLOW,
    Rgb565::CSS_BLUE,
];
const BLACK: u16 = 0;
const MIN_RADIUS: u8 = 0;
const MAX_RADIUS: u8 = 120;
const SCREEN_CENTER: Point = Point::new(MAX_RADIUS as i32, MAX_RADIUS as i32);
const RADIUS_TO_DIAMETER_FACTOR: u8 = 2;
const ENCODER_POLLING_TIMER_MS: u8 = 5;

static SPI_BUS: Mutex<RefCell<Option<Spi<'static, Blocking>>>> = Mutex::new(RefCell::new(None));
static HY040: Mutex<RefCell<Option<Hy040<Input<'static>>>>> = Mutex::new(RefCell::new(None));
static HY040_TIMER: Mutex<RefCell<Option<PeriodicTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));
static CIRCLE_RADIUS: AtomicI32 = AtomicI32::new(MIN_RADIUS as i32);

#[main]
fn main() -> ! {
    // generator version: 0.3.1
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let mut delay = Delay::new();

    // Switches
    let (mut hy040_switch, mut boot_button) = init_switches(peripherals.GPIO6, peripherals.GPIO0);

    // Encoder
    let hy040 = init_hy040(peripherals.GPIO4, peripherals.GPIO5);
    // Interrupt Timer for polling the hy040 encoder
    let mut encoder_timer = timer::PeriodicTimer::new(timg1.timer0);
    encoder_timer.enable_interrupt(true);
    encoder_timer.set_interrupt_handler(encoder_isr);

    // SPI Bus
    let spi = spi_bus::init_spi_bus(peripherals.SPI2, peripherals.GPIO12, peripherals.GPIO13);

    // Mutexes setup
    critical_section::with(|cs| {
        SPI_BUS.borrow_ref_mut(cs).replace(spi);
        HY040.borrow_ref_mut(cs).replace(hy040);
        HY040_TIMER.borrow_ref_mut(cs).replace(encoder_timer);

        // Start timer for encoder polling
        let mut timer = HY040_TIMER.borrow_ref_mut(cs);
        if let Some(timer) = timer.as_mut() {
            timer
                .start(Duration::from_millis(ENCODER_POLLING_TIMER_MS as u64))
                .unwrap();
        }
    });

    // Screen
    let mut display_driver = screen::init_screen(peripherals.GPIO10, peripherals.GPIO3, &SPI_BUS);
    let mut rst = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    // Reset the whole display
    display_driver.reset(&mut rst, &mut delay).unwrap();
    // Initialise the screen
    display_driver.init_with_addr_mode(&mut delay).unwrap();
    // Fill the screen with black pixels
    display_driver.fill(BLACK);

    // Iterator to cycle through the color list.
    // We want to loop over the list when it reaches its end.
    let mut iter = COLOR_LIST.iter().cycle();

    // Shape
    let mut circle = init_background(&mut display_driver, &mut iter);

    // Program loop
    loop {
        // check the switched and change color / reset circle
        switch_handler(&mut boot_button, &mut hy040_switch, &mut circle, &mut iter);

        // We retrieve the radius and adjust it so it does not overflow or goes out
        // of the screen's resolution. This is to avoid panic if values go negative as well as
        // stopping the encoder of increasing the value when it should not have any effect.
        let mut radius = CIRCLE_RADIUS.load(core::sync::atomic::Ordering::Relaxed);
        radius = radius.clamp(MIN_RADIUS as i32, MAX_RADIUS as i32);
        CIRCLE_RADIUS.store(radius, core::sync::atomic::Ordering::Relaxed);

        // Reset the screen as black
        display_driver.fill(BLACK);

        // Adjust the circle's size based on the new radius
        circle.primitive.diameter = radius as u32 * RADIUS_TO_DIAMETER_FACTOR as u32;
        // We need to adjust the top left based on the new size to keep the circle centered.
        circle.primitive.top_left = Point::new(SCREEN_CENTER.x - radius, SCREEN_CENTER.y - radius);
        // Draw and display
        circle.draw(&mut display_driver).unwrap();
        display_driver.flush().unwrap();
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}

#[inline]
fn switch_handler<'a>(
    boot_button: &mut DebouncedSwitch<Input<'a>, debounce::Debouncer>,
    hy040_switch: &mut DebouncedSwitch<Input<'a>, debounce::Debouncer>,
    circle: &mut Styled<Circle, PrimitiveStyle<Rgb565>>,
    color_iter: &mut Cycle<Iter<'a, Rgb565>>,
) {
    // If boot button is pressed, we reset the radius of the circle.
    if boot_button.has_been_pressed().unwrap() {
        println!("Reset radius");
        CIRCLE_RADIUS.store(MIN_RADIUS as i32, core::sync::atomic::Ordering::Relaxed);
    }

    // If the encoder button has been pressed, we change the circle's background.
    if hy040_switch.has_been_pressed().unwrap() {
        println!("Changing color");
        if let Some(color) = color_iter.next() {
            circle.style.fill_color = Some(*color);
        };
    }
}

fn init_switches<'a>(
    hy040_sw_pin: GPIO6<'a>,
    boot_sw_pin: GPIO0<'a>,
) -> (
    DebouncedSwitch<Input<'a>, debounce::Debouncer>,
    DebouncedSwitch<Input<'a>, debounce::Debouncer>,
) {
    // Switches
    let boot_button = switch::Switch::new(
        Input::new(boot_sw_pin, InputConfig::default().with_pull(Pull::Up)),
        PinState::Low,
    )
    .with_debounce(debounce::Debouncer::default());

    let hy040_switch = switch::Switch::new(
        Input::new(hy040_sw_pin, InputConfig::default().with_pull(Pull::Up)),
        PinState::Low,
    )
    .with_debounce(debounce::Debouncer::default());

    (hy040_switch, boot_button)
}

fn init_hy040<'a>(clk: GPIO4<'a>, dt: GPIO5<'a>) -> Hy040<Input<'a>> {
    let config = InputConfig::default().with_pull(Pull::Up);
    let clk = Input::new(clk, config);
    let dt = Input::new(dt, config);
    encoder::Hy040::new(clk, dt)
}

fn init_background<'a>(
    display_driver: &mut DisplayDriver,
    color_iter: &mut Cycle<Iter<'a, Rgb565>>,
) -> Styled<Circle, PrimitiveStyle<Rgb565>> {
    let radius = CIRCLE_RADIUS.load(core::sync::atomic::Ordering::Relaxed);
    let top_left = Point::new(SCREEN_CENTER.x - radius, SCREEN_CENTER.y - radius);
    let mut circle_style = PrimitiveStyle::default();
    if let Some(start_color) = color_iter.next() {
        circle_style = PrimitiveStyle::with_fill(*start_color);
    };
    let circle = Circle::new(top_left, radius as u32 * RADIUS_TO_DIAMETER_FACTOR as u32)
        .into_styled(circle_style);
    circle.draw(display_driver).unwrap();
    display_driver.flush().unwrap();
    circle
}

#[handler]
#[ram]
fn encoder_isr() {
    critical_section::with(|cs| {
        // Retreive objects from mutexes
        let mut hy040 = HY040.borrow_ref_mut(cs);
        let mut timer = HY040_TIMER.borrow_ref_mut(cs);

        // If we have retrieved them,
        if let (Some(hy040), Some(timer)) = (hy040.as_mut(), timer.as_mut()) {
            // Check the direction in which the encoder is rotated
            match hy040.encode() {
                Direction::Clockwise => {
                    // Increase the radius
                    CIRCLE_RADIUS.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
                }
                Direction::CounterClockwise => {
                    // Deacrease the radius
                    CIRCLE_RADIUS.fetch_sub(1, core::sync::atomic::Ordering::Relaxed)
                }
                // Do nothing
                Direction::Rest => 0,
            };
            // Clear the timer interrupt to allow for a new cycle, otherwise it triggers infinitely.
            timer.clear_interrupt();
        }
    });
}
