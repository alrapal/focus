use embedded_hal::digital::{Error, ErrorKind, ErrorType, InputPin, PinState};

use crate::debounce::{self, DebounceState};

/*************************************/
/*************************************/
/******** TRAITS AND ENUMS ***********/
/*************************************/
/*************************************/

/// ## Description
///
/// Trait defining common switch behaviour
pub trait Pressable {
    fn get_current_state(&mut self) -> SwitchState;
    fn has_been_pressed(&mut self) -> Result<bool, SwitchError>;
}

/// ## Description
///
/// Possible switch states.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SwitchState {
    Pressed,    // The switch is pressed
    Released,   // The switch is released
    Transition, // The switch is transitioning from a Pressed state to a Release state or vice versa
    Faulty,     // The switch is faulty
}

/// ## Description
///
/// Easy conversion from a switch state to a boolean.
/// True == Pressed
/// False == Not pressed (any other state)
impl From<SwitchState> for bool {
    fn from(value: SwitchState) -> Self {
        matches!(value, SwitchState::Pressed)
    }
}

/// ## Description
///
/// Possible errors related to switches
#[derive(Debug, PartialEq)]
pub enum SwitchError {
    ReadPinState,
}

/*************************************/
/*************************************/
/* EMBEDDED HAL TRAIT IMPLEMENTATION */
/*************************************/
/*************************************/

impl Error for SwitchError {
    fn kind(&self) -> ErrorKind {
        match self {
            SwitchError::ReadPinState => ErrorKind::Other,
        }
    }
}

impl<PIN> ErrorType for Switch<PIN>
where
    PIN: InputPin,
{
    type Error = SwitchError;
}

/*************************************/
/*************************************/
/******** CONCRETE SWITCHES **********/
/*************************************/
/*************************************/

/********* SIMPLE SWITCH *************/

/// ## Description
///
/// A simple switch with no specific capabilities.
/// Implements the Pressable trait.
///
/// ## Example
///
/// See unit tests for example of use.
///
#[derive(Debug, PartialEq)]
pub struct Switch<PIN>
where
    PIN: InputPin,
{
    pin: PIN,
    pressed_state: PinState,
    last_state: SwitchState,
}

/********* IMPLEMENTATION *************/

impl<PIN> Switch<PIN>
where
    PIN: InputPin,
{
    /// ## Description
    ///
    /// Create a new switch connected to the given input pin, which inteprets the pin's state based
    /// on the expected pressed state.
    ///
    /// ## Parameters
    /// - `pin`: A gpio pin implementing `embedded_hal::digital::InputPin`
    /// - `pressed_state`: The state for which the switch is considered pressed (`PressState::High` or `::Low`)
    ///
    /// ## Return
    /// - Switch
    pub fn new(pin: PIN, pressed_state: PinState) -> Self {
        Switch {
            pin,
            pressed_state,
            last_state: SwitchState::Released,
        }
    }

    /// ## Description
    ///
    /// Add a debouncer to a simple switch. The functions of the switch are filtered through the debouncer.
    ///
    /// ## Parameters
    /// - `debouncer`: An object implementing the `hl_driver::debounce::Debounce` trait.  
    ///
    /// ## Return
    /// - DebouncedSwitch
    pub fn with_debounce<D>(self, debouncer: D) -> DebouncedSwitch<PIN, D>
    where
        D: debounce::Debounce,
    {
        DebouncedSwitch {
            switch: self,
            debouncer,
        }
    }
}

impl<PIN> Pressable for Switch<PIN>
where
    PIN: InputPin,
{
    /// ## Description
    ///
    /// Return the state of the switch when the function is invoqued.
    ///
    /// ## Return
    /// SwitchState:
    /// - Pressed
    /// - Released
    /// - Faulty
    ///
    /// (The Transition state is not returned since there is no debouncing).
    #[inline]
    fn get_current_state(&mut self) -> SwitchState {
        match self.pin.is_high() {
            Ok(b) => {
                if b == bool::from(self.pressed_state) {
                    SwitchState::Pressed
                } else {
                    SwitchState::Released
                }
            }
            Err(_) => SwitchState::Faulty,
        }
    }

    /// ## Description
    ///
    /// Return if the switch has been pressed since the last use of this method.
    ///
    /// ## Return
    /// *Result<bool, SwitchError>*
    /// - `bool`: `true` if the switch has been pressed, `false` otherwise
    /// -  `SwitchError::ReadPinState`: an error occured when reading the gpio pin of the switch
    #[inline]
    fn has_been_pressed(&mut self) -> Result<bool, SwitchError> {
        let current_state = self.get_current_state();
        match current_state {
            SwitchState::Faulty => Err(SwitchError::ReadPinState),
            _ => {
                let was_pressed = self.last_state != SwitchState::Pressed
                    && current_state == SwitchState::Pressed;
                self.last_state = current_state;
                Ok(was_pressed)
            }
        }
    }
}

/********* DEBOUNCED SWITCH *************/

/// ## Description
///
/// A switch with debouncing capabilities.
/// Implements the Pressable trait.
///
/// ## Example
///
/// See unit tests for example of use.
///
#[derive(Debug, PartialEq)]
pub struct DebouncedSwitch<PIN, D>
where
    PIN: InputPin,
    D: debounce::Debounce,
{
    switch: Switch<PIN>,
    debouncer: D,
}

/********* IMPLEMENTATION *************/

impl<PIN, D> Pressable for DebouncedSwitch<PIN, D>
where
    PIN: InputPin,
    D: debounce::Debounce,
{
    /// ## Description
    ///
    /// Return the state of the switch when the function is invoqued.
    ///
    /// ## Return
    /// SwitchState:
    /// - Pressed
    /// - Released
    /// - Transition (debouncing is ongoing)
    /// - Faulty
    #[inline]
    fn get_current_state(&mut self) -> SwitchState {
        match self.switch.pin.is_high() {
            Ok(b) => {
                if b == bool::from(self.switch.pressed_state) {
                    self.debouncer.debounce(true);
                } else {
                    self.debouncer.debounce(false);
                }

                match self.debouncer.get_state() {
                    debounce::DebounceState::Loaded => SwitchState::Pressed,
                    DebounceState::Transition => SwitchState::Transition,
                    DebounceState::Unloaded => SwitchState::Released,
                }
            }
            Err(_) => SwitchState::Faulty,
        }
    }

    /// ## Description
    ///
    /// Return if the switch has been pressed since the last use of this method.
    ///
    /// This takes into account the debouncing.
    ///
    /// ## Return
    /// *Result<bool, SwitchError>*
    /// - `bool`: `true` if the switch has been pressed, `false` otherwise
    /// - `SwitchError::ReadPinState`: an error occured when reading the gpio pin of the switch
    #[inline]
    fn has_been_pressed(&mut self) -> Result<bool, SwitchError> {
        let current_state = self.get_current_state();
        match current_state {
            SwitchState::Faulty => Err(SwitchError::ReadPinState),
            _ => {
                let was_pressed = self.switch.last_state != SwitchState::Pressed
                    && current_state == SwitchState::Pressed;
                self.switch.last_state = current_state;
                Ok(was_pressed)
            }
        }
    }
}

/*************************************/
/*************************************/
/************** TESTS ****************/
/*************************************/
/*************************************/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debounce;
    use crate::test_utils;

    #[inline(never)]
    #[test]
    fn test_switch_get_state() {
        // Pull Up switch with Low level when pressed
        let pressed_state = PinState::Low;
        // Mocked pin with non faulty state and a reading that sets the switch as released.
        let pin = test_utils::MockedGpioPin {
            state: !pressed_state,
            fault: false,
        };
        // Object under test
        let mut switch = Switch::new(pin, pressed_state);

        // Should be released
        assert_eq!(SwitchState::Released, switch.get_current_state());
        // State of the pin becomes pressed
        switch.pin.state = PinState::Low;
        // Should be pressed
        assert_eq!(SwitchState::Pressed, switch.get_current_state());
        // Switch reading is faulty
        switch.pin.fault = true; // simulate an error when reading the pin
        // Sould be faulty
        assert_eq!(SwitchState::Faulty, switch.get_current_state());
    }

    #[inline(never)]
    #[test]
    fn test_simple_switch_has_been_pressed() {
        // Pull Up switch with Low level when pressed
        let pressed_state = PinState::Low;
        // Mocked pin with non faulty state and a reading that sets the switch as released.
        let pin = test_utils::MockedGpioPin {
            state: !pressed_state,
            fault: false,
        };
        // Object under test
        let mut switch = Switch::new(pin, pressed_state);

        // Should be released
        assert_eq!(SwitchState::Released, switch.last_state);
        // State of the pin becomes pressed
        switch.pin.state = PinState::Low;

        // When checking if the button has been pressed,
        // it should be true since the state has changed since last check
        assert_eq!(
            true,
            switch
                .has_been_pressed()
                .expect("Problem when reading the pin")
        );

        // Should not be considered pressed since state did not change
        assert_eq!(
            false,
            switch
                .has_been_pressed()
                .expect("Problem when reading the pin")
        );

        // State of the pin becomes released
        switch.pin.state = PinState::High;
        // It should still be false still since the button has been released
        assert_eq!(
            false,
            switch
                .has_been_pressed()
                .expect("Problem when reading the pin")
        );

        // State of the pin becomes pressed again
        switch.pin.state = PinState::Low;
        // And this should be true again since there was a state change between checks
        assert_eq!(
            true,
            switch
                .has_been_pressed()
                .expect("Problem when reading the pin")
        );
    }

    #[inline(never)]
    #[test]
    fn test_debounced_switch_get_state() {
        // Debouncer implementing the Debounce trait
        let debouncer = debounce::Debouncer::default();

        // Pull Up switch with Low level when pressed
        let pressed_state = PinState::Low;
        // Mocked pin with non faulty state and a reading that sets the switch as released.
        let pin = test_utils::MockedGpioPin {
            state: !pressed_state,
            fault: false,
        };
        // Object under test
        let mut db_switch = Switch::new(pin, pressed_state).with_debounce(debouncer);

        // Should start in release state
        assert_eq!(SwitchState::Released, db_switch.get_current_state());

        // Set the pin to the pressed state to simulate press action
        db_switch.switch.pin.state = PinState::Low;

        // The first ticks should be a transition state
        for _ in 0..2 {
            assert_eq!(SwitchState::Transition, db_switch.get_current_state())
        }

        // The 3th tick should consider a press
        assert_eq!(SwitchState::Pressed, db_switch.get_current_state());

        //? Maintaining the button after that is no a pressed state anymore, is it what we want?
        assert_eq!(SwitchState::Transition, db_switch.get_current_state());

        // Simulate the release of the pin
        db_switch.switch.pin.state = PinState::High;

        // Empty debounce register during 7 tickes and should be tansition state
        for _ in 0..7 {
            assert_eq!(SwitchState::Transition, db_switch.get_current_state())
        }
        // After 7 ticks of transition, the state released state is acknowledged
        assert_eq!(SwitchState::Released, db_switch.get_current_state());
    }

    #[inline(never)]
    #[test]
    fn test_debounced_switch_has_been_pressed() {
        // Pull Up switch with Low level when pressed
        let pressed_state = PinState::Low;
        // Mocked pin with non faulty state and a reading that sets the switch as released.
        let pin = test_utils::MockedGpioPin {
            state: !pressed_state,
            fault: false,
        };
        // Object under test
        let mut db_switch =
            Switch::new(pin, pressed_state).with_debounce(debounce::Debouncer::default());

        // State of the pin becomes pressed
        db_switch.switch.pin.state = PinState::Low;

        // When checking if the button has been pressed,
        // first ticks should not be detected has pressed until we have a clean signal
        for _tick in 0..2 {
            assert_eq!(
                false,
                db_switch
                    .has_been_pressed()
                    .expect("Problem when reading the pin")
            );
        }

        // The next tick should garantee the signal is stable which ensure that the debounced switch has been pressed
        assert_eq!(
            true,
            db_switch
                .has_been_pressed()
                .expect("Problem when reading the pin")
        );

        // State of the pin becomes released
        db_switch.switch.pin.state = PinState::High;

        // It should still be false still since the state is transitioning
        assert_eq!(
            false,
            db_switch
                .has_been_pressed()
                .expect("Problem when reading the pin")
        );
    }
}
