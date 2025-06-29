const DEBOUNCE_MASK: u8 = 0x07;
const RELEASED_MASK: u8 = 0x00;

/// ## Description
///
/// Trait defining debouncing behaviours
pub trait Debounce {
    fn get_state(&self) -> DebounceState;
    fn debounce(&mut self, state: bool);
}

/// ## Description
///
/// Possible state for a debouncer
#[derive(Debug, PartialEq)]
pub enum DebounceState {
    Loaded,
    Unloaded,
    Transition,
}

/// ## Description
///
/// Allow rapid conversion between u8 and the Debounce state.
/// Useful for converting the state of a register.
impl From<u8> for DebounceState {
    fn from(value: u8) -> Self {
        match value {
            DEBOUNCE_MASK => DebounceState::Loaded,
            RELEASED_MASK => DebounceState::Unloaded,
            _ => DebounceState::Transition,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
/// ## Description
///
/// Debouncer struct implementing the debouncing trait based on a u8 register
///
/// ## Example
///
/// ```rust
///     use hl_driver::debounce::{Debouncer, DebounceState, Debounce};
///     // Create a debouncer with en empty register
///     let mut debouncer = Debouncer::default();
///     // Perform one register manipulation based on the provided boolean. For instance a gpio state.
///     debouncer.debounce(true);
///     // Retrieve the DebounceState. When the register is full (3 ticks), the state will be Loaded.
///     let state = debouncer.get_state();
///     assert_eq!(DebounceState::Transition, state);
/// ```
#[derive(Default)]
pub struct Debouncer {
    register: u8,
}

impl Debounce for Debouncer {
    fn debounce(&mut self, state: bool) {
        self.register = (self.register << 1) | state as u8;
    }

    fn get_state(&self) -> DebounceState {
        DebounceState::from(self.register)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline(never)]
    #[test]
    fn test_debouncer() {
        let mut debouncer = Debouncer::default();

        // 3 first ticks filling the register
        for _ in 0..2 {
            debouncer.debounce(true);
            assert_eq!(DebounceState::Transition, debouncer.get_state());
        }

        // 3th tick should fill register and reach loaded state
        debouncer.debounce(true);
        assert_eq!(DebounceState::Loaded, debouncer.get_state());

        // 7 more ticks emptying the register
        for _ in 0..7 {
            debouncer.debounce(false);
            assert_eq!(DebounceState::Transition, debouncer.get_state());
        }

        // 8th tick should empty the register and reach unloaded state
        debouncer.debounce(false);
        assert_eq!(DebounceState::Unloaded, debouncer.get_state());
    }
}
