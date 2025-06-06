use esp_hal::gpio::{GpioPin, Input, InputConfig, Pull};

pub fn init_boot_button(pin: GpioPin<0>) -> Input<'static> {
    let config = InputConfig::default().with_pull(Pull::Up);
    Input::new(pin, config)
}
