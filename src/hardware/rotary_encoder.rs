use core::fmt::Debug;

use esp_hal::gpio::{Event, Input, Level};

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    CCW,
    CW,
    Rest,
}

#[allow(dead_code)]
pub trait State {
    fn state(&self) -> u8;
    fn set_state(&mut self, state: u8) -> u8;
}

#[allow(dead_code)]
pub trait Encode: State {
    fn clk(&self) -> &Input;
    fn dt(&self) -> &Input;

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
            13 => Direction::CW,
            4 => Direction::CW,
            2 => Direction::CW,
            11 => Direction::CW,
            14 => Direction::CCW,
            8 => Direction::CCW,
            1 => Direction::CCW,
            7 => Direction::CCW,
            _ => Direction::Rest,
        }
    }
}

const DEFAULT_STATE: u8 = 0b11;

#[derive(Debug)]
pub struct EncoderWithoutSwitch<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    state: u8,
}

#[allow(dead_code)]
impl<'a> EncoderWithoutSwitch<'a> {
    pub fn new(clk: Input<'a>, dt: Input<'a>) -> Self {
        EncoderWithoutSwitch {
            clk,
            dt,
            state: DEFAULT_STATE,
        }
    }

    pub fn add_switch(self, sw: Input<'a>) -> EncoderWithSwitch<'a> {
        EncoderWithSwitch {
            clk: self.clk,
            dt: self.dt,
            state: self.state,
            sw,
        }
    }
}

impl State for EncoderWithoutSwitch<'_> {
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

impl Encode for EncoderWithoutSwitch<'_> {
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
pub struct EncoderWithSwitch<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    sw: Input<'a>,
    state: u8,
}

#[allow(dead_code)]
impl<'a> EncoderWithSwitch<'a> {
    pub fn add_switch_listener(self, event: Event) -> EncoderSwitchEventListener<'a> {
        let mut tmp = EncoderSwitchEventListener {
            clk: self.clk,
            dt: self.dt,
            sw: self.sw,
            state: self.state,
        };
        tmp.sw.listen(event);
        tmp
    }

    pub fn remove_switch(self) -> (EncoderWithoutSwitch<'a>, Input<'a>) {
        let tmp = EncoderWithoutSwitch {
            clk: self.clk,
            dt: self.dt,
            state: self.state,
        };
        let input = self.sw;
        (tmp, input)
    }

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
impl State for EncoderWithSwitch<'_> {
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

impl Encode for EncoderWithSwitch<'_> {
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
pub struct EncoderSwitchEventListener<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    sw: Input<'a>,
    state: u8,
}

#[allow(dead_code)]
impl<'a> EncoderSwitchEventListener<'a> {
    pub fn remover_switch_listener(mut self) -> EncoderWithSwitch<'a> {
        self.sw.unlisten();
        EncoderWithSwitch {
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
impl State for EncoderSwitchEventListener<'_> {
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

impl Encode for EncoderSwitchEventListener<'_> {
    #[inline]
    fn clk(&self) -> &Input {
        &self.clk
    }

    #[inline]
    fn dt(&self) -> &Input {
        &self.dt
    }
}
