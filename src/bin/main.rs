#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicI32},
};
use critical_section::Mutex;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, WebColors},
    primitives::{Circle, PrimitiveStyle},
    Drawable,
};
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Io, Level, Output, OutputConfig, Pull},
    handler, main, ram,
    spi::master::Spi,
    time::{Duration, Instant},
    timer::{self, timg::TimerGroup, PeriodicTimer},
    Blocking,
};
use esp_println::println;
use focus::hardware::{
    button,
    encoder::{self, Encode},
    screen, spi_bus,
};

#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", e);
    loop {}
}
// Static modules in Mutex for safe access between threads / interrupts
static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static SPI_BUS: Mutex<RefCell<Option<Spi<'static, Blocking>>>> = Mutex::new(RefCell::new(None));
static HY_040: Mutex<RefCell<Option<encoder::EncoderListener>>> =
    Mutex::new(RefCell::new(None));
static DEBOUNCE_TIMER: Mutex<RefCell<Option<Instant>>> = Mutex::new(RefCell::new(None));
static ENCODER_TIMER: Mutex<RefCell<Option<PeriodicTimer<Blocking>>>> =
    Mutex::new(RefCell::new(None));

// Atomic for safe access between threads
static COUNTER: AtomicI32 = AtomicI32::new(0);
static SW_PRESSED: AtomicBool = AtomicBool::new(false);
static BOOT_PRESSED: AtomicBool = AtomicBool::new(false);

// Constant values
const DELAY_LOOP_MS: u64 = 10;
const DEBOUNCE_MS: u64 = 200;
const ENCODER_TIMER_MS: u64 = 5;
const COLOR_LIST: [Rgb565; 3] = [Rgb565::CSS_RED, Rgb565::CSS_GREEN, Rgb565::CSS_BLUE];
const SCREEN_WIDTH_PIXELS: u8 = 240;
const FACTOR_TWO: u8 = 2;
const MIN_COUNTER: u8 = 0;
const MAX_COUNTER: u8 = SCREEN_WIDTH_PIXELS / FACTOR_TWO;
const BLACK_U16: u16 = 0;

#[main]
fn main() -> ! {
    // generator version: 0.3.1
    let mut last_counter = 0_i32;

    // Esp32s3 configuration
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let timg1 = TimerGroup::new(peripherals.TIMG1);

    //IO mulitplexer for gpio interrupt
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(button_handler);

    // Encoder
    let mut encoder_timer = timer::PeriodicTimer::new(timg1.timer0);
    encoder_timer.enable_interrupt(true);
    encoder_timer.set_interrupt_handler(encoder_handler);

    // Debounce time
    let debounce_timer = Instant::now();

    // Button
    let mut boot_button = button::init_boot_button(peripherals.GPIO0);
    boot_button.listen(esp_hal::gpio::Event::FallingEdge);

    // HY-040
    let config = InputConfig::default().with_pull(Pull::Up);
    let clk = Input::new(peripherals.GPIO4, config);
    let dt = Input::new(peripherals.GPIO5, config);
    let sw = Input::new(peripherals.GPIO6, config);
    let hy_040 = encoder::Encoder::new(clk, dt)
        .add_switch(sw)
        .add_switch_listener(esp_hal::gpio::Event::FallingEdge);

    // SPI Bus
    let spi = spi_bus::init_spi_bus(peripherals.SPI2, peripherals.GPIO12, peripherals.GPIO13);

    // Mutexes setup: we place all previous elements in their respective mutexes
    critical_section::with(|cs| {
        DEBOUNCE_TIMER.borrow_ref_mut(cs).replace(debounce_timer);
        BUTTON.borrow_ref_mut(cs).replace(boot_button);
        HY_040.borrow_ref_mut(cs).replace(hy_040);
        SPI_BUS.borrow_ref_mut(cs).replace(spi);
        ENCODER_TIMER.borrow_ref_mut(cs).replace(encoder_timer);

        // Start the timer after it's been placed in Mutex, to start triggering the update for the encoder
        let mut encoder_timer = ENCODER_TIMER.borrow_ref_mut(cs);
        if let Some(encoder_timer) = encoder_timer.as_mut() {
            encoder_timer
                .start(Duration::from_millis(ENCODER_TIMER_MS))
                .unwrap();
        }
    });

    // Screen
    let mut delay = Delay::new();
    let mut display_driver = screen::init_screen(peripherals.GPIO10, peripherals.GPIO3, &SPI_BUS);
    let mut rst = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());
    display_driver.reset(&mut rst, &mut delay).unwrap();
    display_driver.init_with_addr_mode(&mut delay).unwrap();
    display_driver.fill(BLACK_U16); // fill the screen buffer with black pixels

    // Iterator to iterate through the color list
    let mut iter = COLOR_LIST.iter().cycle();

    // Shape
    let mut radius = MIN_COUNTER as u32;
    let center = Point::new(MAX_COUNTER as i32, MAX_COUNTER as i32);
    let mut top_left = Point::new(center.x - radius as i32, center.y - radius as i32);
    let first_color = iter.next().expect("Could not retrieve first color");
    let circle_style = PrimitiveStyle::with_fill(*first_color);
    let mut circle = Circle::new(top_left, radius * (FACTOR_TWO as u32)).into_styled(circle_style);
    circle.draw(&mut display_driver).unwrap(); // draw command writes the given drawable in the buffer

    // Display the buffer on the screen
    display_driver.flush().unwrap();

    // Start loop delay
    let mut loop_timer = Instant::now();

    loop {
        // Handle encoder switch pressed
        // swap return current value and replaces it with provided one
        if SW_PRESSED.swap(false, core::sync::atomic::Ordering::Relaxed) {
            // reset counter if exceed screen min max bound
            COUNTER.swap(0, core::sync::atomic::Ordering::Relaxed);
        }

        // Handle boot button pressed
        // swap return current value and replaces it with provided one
        if BOOT_PRESSED.swap(false, core::sync::atomic::Ordering::Relaxed) {
            // Set the circle background color to the next color in the list
            if let Some(color) = iter.next() {
                circle.style.fill_color = Some(*color);
            };
        }

        // Update counter boundaries if
        let counter = COUNTER.load(core::sync::atomic::Ordering::Relaxed);
        if counter > MAX_COUNTER as i32 {
            // store saves the provided value into atomic
            COUNTER.store(MAX_COUNTER as i32, core::sync::atomic::Ordering::Relaxed);
        } else if counter < MIN_COUNTER as i32 {
            // store saves the provided value into atomic
            COUNTER.store(MIN_COUNTER as i32, core::sync::atomic::Ordering::Relaxed);
        }

        // If there is a change between the current counter and the last counter checked
        // load returns the current value stored in atomic
        let counter = COUNTER.load(core::sync::atomic::Ordering::Relaxed);
        if counter != last_counter {
            // update last counter
            last_counter = counter;
            // set the circle radius to the value
            radius = counter as u32;
        }

        // If the delay for the loop is passed, we update the circle with the new radius and display
        if loop_timer.elapsed().as_millis() >= DELAY_LOOP_MS {
            loop_timer = Instant::now(); // reset timer
            display_driver.fill(BLACK_U16);
            top_left = Point::new(center.x - radius as i32, center.y - radius as i32);
            circle.primitive.diameter = radius * FACTOR_TWO as u32;
            circle.primitive.top_left = top_left;
            circle.draw(&mut display_driver).unwrap();
            display_driver.flush().unwrap();
        }
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/src/bin
}

#[handler]
#[ram]
fn button_handler() {
    critical_section::with(|cs| {
        // Take ownership of the timer to check if debounce is happening
        let mut debounce_timer = DEBOUNCE_TIMER.borrow_ref_mut(cs);

        // Check if the elapsed time is enough
        if let Some(last) = debounce_timer.as_ref() {
            if last.elapsed().as_millis() < DEBOUNCE_MS {
                return;
            }
        }

        let now = Instant::now(); // Save timestamp when entering the CS
                                  // Borrow mutexes values now that we know
        let mut button = BUTTON.borrow_ref_mut(cs);
        let mut hy_040 = HY_040.borrow_ref_mut(cs);

        // Reset debounce timer
        debounce_timer.replace(now);

        // Handler the boot button by raising the flag handled in main
        if let Some(button) = button.as_mut() {
            if button.is_interrupt_set() {
                // store saves the provided value into atomic
                BOOT_PRESSED.store(true, core::sync::atomic::Ordering::Relaxed);
                // We need to clear interrupt once handled
                button.clear_interrupt();
            }
        }

        // Handle the switch attached to the encoder
        if let Some(hy_040) = hy_040.as_mut() {
            if hy_040.has_been_pressed() {
                // store saves the provided value into atomic
                SW_PRESSED.store(true, core::sync::atomic::Ordering::Relaxed);
            }
        }
    });
}

#[handler]
#[ram]
fn encoder_handler() {
    critical_section::with(|cs| {
        // Take ownership of the different mutexes necessary for this handler
        let mut hy_040 = HY_040.borrow_ref_mut(cs);
        let mut timer = ENCODER_TIMER.borrow_ref_mut(cs);

        // Update the counter based on the direction provided by the encoder
        if let (Some(hy_040), Some(timer)) = (hy_040.as_mut(), timer.as_mut()) {
            match hy_040.update() {
                encoder::Direction::Clockwise => {
                    // fetch add increase with provided value
                    COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
                }
                encoder::Direction::CounterClockwise => {
                    // fetch sub decrease with provided value
                    COUNTER.fetch_sub(1, core::sync::atomic::Ordering::Relaxed)
                }
                encoder::Direction::Rest => 0,
            };

            // We need to clear interrupt once handled
            timer.clear_interrupt();
        }
    })
}
