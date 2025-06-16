use core::fmt::Debug;

use esp_hal::gpio::Input;

#[derive(Debug)]
pub struct Hy040<'a> {
    clk: Input<'a>,
    dt: Input<'a>,
    sw: Input<'a>,
    state: i8,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    CCW,
    CW,
    Rest,
}

impl<'a> Hy040<'a> {
    pub fn new(clk: Input<'a>, dt: Input<'a>, sw: Input<'a>) -> Hy040<'a> {
        let mut tmp = Hy040 {
            clk,
            dt,
            sw,
            state: 0x3,
        };
        tmp.sw.listen(esp_hal::gpio::Event::RisingEdge);
        tmp
    }

    pub fn update(&mut self) -> Direction {
        // println!("Update");
        // Move old state to the
        self.state <<= 2;
        if self.clk.is_high() {
            self.state |= 0x1
        };
        if self.dt.is_high() {
            self.state |= 0x2
        };
        self.state &= 0x0F;
        // Here we have a 4 bits values which represents the last state and the current one
        match self.state {
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

    #[inline]
    pub fn is_any_interrupt_set(&mut self) -> bool {
        self.dt.is_interrupt_set() || self.clk.is_interrupt_set()
    }

    #[inline]
    pub fn clear_interrupts(&mut self) {
        self.dt.clear_interrupt();
        self.clk.clear_interrupt();
    }

    #[inline]
    pub fn is_dt_interrupt_set(&self) -> bool {
        self.dt.is_interrupt_set()
    }

    #[inline]
    pub fn is_clk_interrupt_set(&self) -> bool {
        self.clk.is_interrupt_set()
    }

    #[inline]
    pub fn is_sw_interrupt_set(&self) -> bool {
        self.sw.is_interrupt_set()
    }

    #[inline]
    pub fn clear_dt_interrupt(&mut self) {
        self.dt.clear_interrupt();
    }

    #[inline]
    pub fn clear_clk_interrupt(&mut self) {
        self.clk.clear_interrupt();
    }

    #[inline]
    pub fn clear_sw_interrupt(&mut self) {
        self.sw.clear_interrupt();
    }
}
