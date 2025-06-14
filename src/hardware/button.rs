use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::peripherals::GPIO0;

pub fn init_boot_button(pin: GPIO0<'static>) -> Input<'static> {
    let config = InputConfig::default().with_pull(Pull::Up);
    Input::new(pin, config)
}
