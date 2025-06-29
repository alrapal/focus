use embedded_hal::digital::{ErrorKind, ErrorType, InputPin, PinState};

/// ## Description
///
/// Mock of a simple gpio pin for unit tests
///
/// ## Example
///
/// ```rust
/// use embedded_hal::digital::PinState;
/// use hl_driver::test_utils::MockedPin;
///
/// let mocked_pin = MockedPin{state: PinState::High, fault: false};
/// ```
///
pub struct MockedGpioPin {
    pub state: PinState,
    pub fault: bool,
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
