use crate::switch::Pressable;
use core::fmt::Debug;
use embedded_hal::digital::InputPin;

// A valid Rest Direction for a HY040 rotary encoder
const DEFAULT_STATE: u8 = 0b11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// ## Description
/// Represent the direction in which the rotary encoder is being rotated.
pub enum Direction {
    CounterClockwise,
    Clockwise,
    Rest,
}

/// ## Description
/// Encoder traits.
pub trait Encode {
    /// ## Description
    /// This function defines a common interface for knowing in what direction an encoder
    /// is being rotated.
    /// ## Return
    /// Direction in which the encoder is being rotated.
    fn encode(&mut self) -> Direction;
}

/// ## Description
/// Represents a simple Rotary Encoder with basic functionnality.
#[derive(Debug)]
pub struct Hy040<INPUT>
where
    INPUT: InputPin,
{
    clk: INPUT,
    dt: INPUT,
    state: u8,
}

impl<INPUT> Hy040<INPUT>
where
    INPUT: InputPin,
{
    /// ## Description
    /// Create a new Encoder from which Direction can be retrieved.
    /// ### Parameters
    /// - clk: the gpio pin connected to the A pin of the Rotary encoder
    /// - dt: the gpio pin connected to the B pin of the Rotary encoder
    /// ### Return
    /// - Encoder
    pub fn new(clk: INPUT, dt: INPUT) -> Self {
        Hy040 {
            clk,
            dt,
            state: DEFAULT_STATE,
        }
    }

    /// ## Description
    /// Add a switch to an Encoder from which switch status can be read.
    /// The switch has to implement the `Pressable` trait.
    /// ### Parameters
    /// - sw: a switch implementing `hl_driver::switch::Pressable`
    /// ### Return
    /// Encoder with switch functionnalities
    pub fn with_switch<SW: Pressable>(self, sw: SW) -> Hy040WithSwitch<INPUT, SW> {
        Hy040WithSwitch {
            encoder: self,
            switch: sw,
        }
    }
}

impl<INPUT> Encode for Hy040<INPUT>
where
    INPUT: InputPin,
{
    /// ## Description
    /// Read the state of the two pins attached to the rotary forming a 2bits state.
    /// The prior state and the current state are combined in a 4 bits value used
    /// to determine the sense of rotation of the encoder.
    /// ## Return
    /// - `Direction`: Direction can be CounterClockwise, Clockwise or Rest.
    #[inline]
    fn encode(&mut self) -> Direction {
        let mut current_state = self.state;
        current_state <<= 2;
        if self.clk.is_high().expect("Should not fail") {
            current_state |= 0x2
        };
        if self.dt.is_high().expect("Should not fail") {
            current_state |= 0x1
        };
        current_state &= 0x0F;
        self.state = current_state;
        // Here we have a 4 bits values which represents the last state and the current one
        match current_state {
            13 => Direction::Clockwise,
            4 => Direction::Clockwise,
            2 => Direction::Clockwise,
            11 => Direction::Clockwise,
            14 => Direction::CounterClockwise,
            8 => Direction::CounterClockwise,
            1 => Direction::CounterClockwise,
            7 => Direction::CounterClockwise,
            _ => Direction::Rest,
        }
    }
}

/// ## Description
/// An Encoder with a switch. See hl_driver::switch module for more details.
/// The encoder implements both the `Encode` trait and the `hl_driver::switch::Pressable` trait.
#[derive(Debug)]
pub struct Hy040WithSwitch<INPUT, SW>
where
    INPUT: InputPin,
    SW: Pressable,
{
    encoder: Hy040<INPUT>,
    switch: SW,
}

impl<INPUT, SW> Pressable for Hy040WithSwitch<INPUT, SW>
where
    INPUT: InputPin,
    SW: Pressable,
{
    /// ## Description
    /// Get the state of the switch when the function is called.
    /// ## Return
    /// - `SwitchState`: Pressed, Released or Transition if debouncer attached to the switch.
    #[inline]
    fn get_current_state(&mut self) -> crate::switch::SwitchState {
        self.switch.get_current_state()
    }

    /// ## Description
    /// Indicate if the switch has been pressed since the last time this method has been called.
    /// Useful in superloop architecture or timer based logic.
    /// ## Return
    /// - `bool`: `true` if the switch has been pressed, false otherwise.
    #[inline]
    fn has_been_pressed(&mut self) -> Result<bool, crate::switch::SwitchError> {
        self.switch.has_been_pressed()
    }
}

impl<INPUT, SW> Encode for Hy040WithSwitch<INPUT, SW>
where
    INPUT: InputPin,
    SW: Pressable,
{
    /// ## Description
    /// (Forwards the `Encode` implementation of the Encoder)
    /// Read the state of the two pins attached to the rotary forming a 2bits state.
    /// The prior state and the current state are combined in a 4 bits value used
    /// to determine the sense of rotation of the encoder.
    /// ## Return
    /// - `Direction`: Direction can be CounterClockwise, Clockwise or Rest.
    #[inline]
    fn encode(&mut self) -> Direction {
        self.encoder.encode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockedGpioPin;
    use embedded_hal::digital::PinState;

    #[inline(never)]
    #[test]
    fn test_encoder_state_should_start_with_default() {
        // Both pin are initiated low and non faulty
        let mocked_clk_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };
        let mocked_dt_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };

        let hy040 = Hy040::new(mocked_clk_pin, mocked_dt_pin);
        // Internal state should be the default state.
        assert_eq!(DEFAULT_STATE, hy040.state);
    }

    #[inline(never)]
    #[test]
    fn test_encoder_four_bits_state() {
        // Clk is Low and dt is High.
        // Both are non faulty.
        let mocked_clk_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };
        let mocked_dt_pin = MockedGpioPin {
            state: PinState::High,
            fault: false,
        };

        let mut hy040 = Hy040::new(mocked_clk_pin, mocked_dt_pin);
        // Starting with 0:
        // - even bits are DT
        // - odd bits are CLK
        // The most significant pair of bits is the previous state
        // The least significant pair of bits is the current state
        let expected_state = 0b1101;
        hy040.encode();

        assert_eq!(expected_state, hy040.state);
    }

    #[inline(never)]
    #[test]
    fn test_encoder_should_return_couterclockwise_dir() {
        // Clk is High and dt is low.
        // Both are non faulty.
        let mocked_clk_pin = MockedGpioPin {
            state: PinState::High,
            fault: false,
        };
        let mocked_dt_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };

        let mut hy040 = Hy040::new(mocked_clk_pin, mocked_dt_pin);

        let dir = hy040.encode();
        assert_eq!(Direction::CounterClockwise, dir);
    }

    #[inline(never)]
    #[test]
    fn test_encoder_should_return_clockwise_dir() {
        // Clk is Low and dt is High.
        // Both are non faulty.
        let mocked_clk_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };
        let mocked_dt_pin = MockedGpioPin {
            state: PinState::High,
            fault: false,
        };

        let mut hy040 = Hy040::new(mocked_clk_pin, mocked_dt_pin);

        let dir = hy040.encode();
        assert_eq!(Direction::Clockwise, dir);
    }

    #[inline(never)]
    #[test]
    fn test_encoder_should_return_rest_dir() {
        // Both pin are initiated low and non faulty
        let mocked_clk_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };
        let mocked_dt_pin = MockedGpioPin {
            state: PinState::Low,
            fault: false,
        };

        let mut hy040 = Hy040::new(mocked_clk_pin, mocked_dt_pin);

        let dir = hy040.encode();
        assert_eq!(Direction::Rest, dir);
    }
}
