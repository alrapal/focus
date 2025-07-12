use embedded_hal::digital::{ErrorKind, ErrorType, InputPin, PinState};

/// ## Description
/// Mock of a simple gpio pin for unit tests
pub struct MockedGpioPin {
    pub state: PinState,
    pub fault: bool,
}

impl Default for MockedGpioPin {
    fn default() -> Self {
        MockedGpioPin {
            state: PinState::Low,
            fault: false,
        }
    }
}

impl ErrorType for MockedGpioPin {
    type Error = ErrorKind;
}

impl InputPin for MockedGpioPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        match self.fault {
            true => Err(ErrorKind::Other),
            false => Ok(bool::from(self.state)),
        }
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        match self.fault {
            true => Err(ErrorKind::Other),
            false => Ok(bool::from(!self.state)),
        }
    }
}
