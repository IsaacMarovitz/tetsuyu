use crate::hw::interrupt::Interrupts;
use crate::components::prelude::*;

pub struct Timer {
    counter: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    pub interrupts: Interrupts,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            interrupts: Interrupts::empty(),
        }
    }

    pub fn div(&self) -> u8 {
        (self.counter >> 8) as u8
    }

    fn tac_bit(tac: u8) -> u16 {
        match tac & 0b0000_0011 {
            0 => 9,
            1 => 3,
            2 => 5,
            _ => 7,
        }
    }

    fn mux(&self, tac: u8) -> bool {
        (tac & 0b0000_0100 != 0) && (self.counter >> Self::tac_bit(tac)) & 1 != 0
    }

    fn tima_inc(&mut self, times: u16) {
        for _ in 0..times {
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0 {
                self.tima = self.tma;
                self.interrupts |= Interrupts::TIMER;
            }
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        let old = self.counter;
        self.counter = self.counter.wrapping_add(cycles as u16);

        if self.tac & 0b0000_0100 != 0 {
            let bit = Self::tac_bit(self.tac) + 1;
            let edges = (self.counter >> bit).wrapping_sub(old >> bit);
            self.tima_inc(edges);
        }
    }
}

impl Memory for Timer {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF04 => (self.counter >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => 0xF8 | self.tac,
            _ => panic!("Read to unsupported timer address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF04 => {
                // Resetting DIV can drop the mux output 1->0 and tick TIMA.
                if self.mux(self.tac) {
                    self.tima_inc(1);
                }
                self.counter = 0;
            }
            0xFF05 => self.tima = v,
            0xFF06 => self.tma = v,
            0xFF07 => {
                // A TAC write that drops the mux output 1->0 also ticks TIMA
                // (covers both the disable case and a frequency change).
                let old_out = self.mux(self.tac);
                self.tac = v & 0x07;
                if old_out && !self.mux(self.tac) {
                    self.tima_inc(1);
                }
            }
            _ => panic!("Write to unsupported timer address ({:#06x})!", a),
        }
    }
}
