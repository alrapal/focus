use core::fmt::Debug;

use esp_hal::gpio::{Event, Input, Level};

const DEFAULT_STATE: u8 = 0b11;

#[derive(Debug, Clone, Copy)]
/// # Description
/// Represent the direction in which the rotary is being moved towards.
pub enum Direction {
    CounterClockwise,
    Clockwise,
    Rest,
}

/// ## Description
/// Trait providing necessary methods to interract with the state of the encoder
#[allow(dead_code)]
pub trait State {
    /// ## Description
    /// Get the state
    fn state(&self) -> u8;

    /// ## Description
    /// Set the state to given value
    /// ### Parameter
    /// - state: new state to set
    /// ### Return
    /// - u8: Previous state
    fn set_state(&mut self, state: u8) -> u8;
}

/// # Description
/// - Provide default implementation for Rotary Encoders.
/// - Define necessary getters for the default update.
#[allow(dead_code)]
pub trait Encode: State {
    /// ## Description
    /// Retreive a handle on the clk pin
    fn clk(&self) -> &Input;

    /// ## Description
    /// Retreive a handle on the dt pin
    fn dt(&self) -> &Input;

    /// ## Description
    /// Reads the current clk and dt pins and compare with the previous state to determine the Direction.
    ///
    /// ### Return
    /// Direction the encoder is being turned toward.
    #[inline]
    fn update(&mut self) -> Direction {
        let mut current_state = self.state();
        current_state <<= 2;
        if self.clk().is_high() {
            current_state |= 0x1
        };
        if self.dt().is_high() {
            current_state |= 0x2
        };
        current_state &= 0x0F;
        self.set_state(current_state);
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
/// Represents a simple Rotary Encoder with basic functionnality.
#[derive(Debug)]
pub struct BasicEncoder<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    state: u8,
}

#[allow(dead_code)]
impl<'a> BasicEncoder<'a> {
    /// ## Description
    /// Create a new Encoder from which Direction can be retrieved.
    ///
    /// ### Parameters
    /// - clk: the gpio pin connected to the A pin of the Rotary encoder
    /// - dt: the gpio pin connected to the B pin of the Rotary encoder
    ///
    /// ### Return
    /// - Encoder
    ///
    /// ### Example
    ///
    /// ```rust
    ///    let basic_encoder = EncoderWithoutSwitch::new(clk, dt);
    /// ```
    ///
    pub fn new(clk: Input<'a>, dt: Input<'a>) -> Self {
        BasicEncoder {
            clk,
            dt,
            state: DEFAULT_STATE,
        }
    }

    /// ## Description
    /// Add a switch to an Encoder from which switch status can be read.
    ///
    /// ### Parameters
    /// - sw: the gpio pin connected to the A pin of the Rotary encoder
    ///
    /// ### Return
    /// Encoder with switch functionnalities
    ///
    /// ### Example
    /// ```rust
    ///    let switch_encoder = BasicEncoder::new(clk, dt).add_switch(sw);
    /// ```
    pub fn add_switch(self, sw: Input<'a>) -> EncoderSwitch<'a> {
        EncoderSwitch {
            clk: self.clk,
            dt: self.dt,
            state: self.state,
            sw,
        }
    }
}

impl State for BasicEncoder<'_> {
    #[inline]
    fn set_state(&mut self, state: u8) -> u8 {
        let temp = self.state;
        self.state = state;
        temp
    }

    #[inline]
    fn state(&self) -> u8 {
        self.state
    }
}

impl Encode for BasicEncoder<'_> {
    #[inline]
    fn clk(&self) -> &Input {
        &self.clk
    }

    #[inline]
    fn dt(&self) -> &Input {
        &self.dt
    }
}

/// Encoder with switch
#[derive(Debug)]
pub struct EncoderSwitch<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    sw: Input<'a>,
    state: u8,
}

#[allow(dead_code)]
impl<'a> EncoderSwitch<'a> {
    /// ## Description
    /// Change the Switch logic of the encoder to base on interrupt logic.
    /// ### Parameters
    /// - evemt: Event from gpio triggering the switch press.
    /// ### Return
    /// - Encoder with gpio listener
    ///
    /// ### Example
    /// ```rust
    ///    let switch_listener_encoder = EncoderWithoutSwitch::new(clk, dt).add_switch(sw).add_switch_listener(Event::FallingEdge);
    /// ```
    pub fn add_switch_listener(self, event: Event) -> EncoderListener<'a> {
        let mut tmp = EncoderListener {
            clk: self.clk,
            dt: self.dt,
            sw: self.sw,
            state: self.state,
        };
        tmp.sw.listen(event);
        tmp
    }

    /// ## Description
    /// Downgrade the encoder to s basic Encoder, allowing to reuse the switch Input pin.
    /// ### Return
    /// - Encoder and Input pin
    ///
    /// ### Example
    /// ```rust    
    ///     let switch_listener_encoder = BasicEncoder::new(clk, dt).add_switch(sw).add_switch_listener(Event::FallingEdge);
    ///     let (simple_encoder, pin) = switch_encoder.remove_switch();
    /// ````
    pub fn remove_switch(self) -> (BasicEncoder<'a>, Input<'a>) {
        let tmp = BasicEncoder {
            clk: self.clk,
            dt: self.dt,
            state: self.state,
        };
        let input = self.sw;
        (tmp, input)
    }

    /// ## Description
    /// Checks if the button is being pressed, based on exected logic level.
    /// 
    /// ### Parameter
    /// - Level: Logic level expected for the switch to be pressed. (Depends on the InputConfig used to configure the gpio connected to the switch.)
    /// 
    /// ### Return
    /// - True if pressed, false otherwise
    ///
    /// ### Example
    /// ```rust    
    ///     let switch_encoder = BasicEncoder::new(clk, dt).add_switch(sw);
    ///     if switch_encoder.is_pressed_with_level(Level::Low) {
    ///         println!("The button is being pressed");
    ///     };
    /// ````
    #[inline]
    pub fn is_pressed_with_level(&self, pressed_level: Level) -> bool {
        if self.sw.is_high() && pressed_level == Level::High {
            true
        } else {
            self.sw.is_low() && pressed_level == Level::Low
        }
    }
}
#[allow(dead_code)]
impl State for EncoderSwitch<'_> {
    #[inline]
    fn set_state(&mut self, state: u8) -> u8 {
        let temp = self.state;
        self.state = state;
        temp
    }

    #[inline]
    fn state(&self) -> u8 {
        self.state
    }
}

impl Encode for EncoderSwitch<'_> {
    #[inline]
    fn clk(&self) -> &Input {
        &self.clk
    }

    #[inline]
    fn dt(&self) -> &Input {
        &self.dt
    }
}

/// Encoder with switch and event listener
#[allow(dead_code)]
#[derive(Debug)]
pub struct EncoderListener<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    sw: Input<'a>,
    state: u8,
}

#[allow(dead_code)]
impl<'a> EncoderListener<'a> {
    pub fn remover_switch_listener(mut self) -> EncoderSwitch<'a> {
        self.sw.unlisten();
        EncoderSwitch {
            clk: self.clk,
            dt: self.dt,
            sw: self.sw,
            state: self.state,
        }
    }

    #[inline]
    pub fn has_been_pressed(&mut self) -> bool {
        if self.sw.is_interrupt_set() {
            self.sw.clear_interrupt();
            true
        } else {
            false
        }
    }
}

#[allow(dead_code)]
impl State for EncoderListener<'_> {
    #[inline]
    fn set_state(&mut self, state: u8) -> u8 {
        let temp = self.state;
        self.state = state;
        temp
    }

    #[inline]
    fn state(&self) -> u8 {
        self.state
    }
}

impl Encode for EncoderListener<'_> {
    #[inline]
    fn clk(&self) -> &Input {
        &self.clk
    }

    #[inline]
    fn dt(&self) -> &Input {
        &self.dt
    }
}
