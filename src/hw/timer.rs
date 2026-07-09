use crate::components::prelude::io;
use super::bus::{BusDir, Chip, Pins, Ticked};
use super::interrupt::Interrupts;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reload {
    None,
    /// Cycle A
    Overflowed {
        dots: u8,
    },
    /// Cycle B
    Reloading {
        dots: u8,
    },
}

pub struct Timer {
    counter: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    /// The overflow/reload phase (see [`Reload`]).
    reload: Reload,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            reload: Reload::None,
        }
    }

    fn tac_bit(tac: u8) -> u16 {
        match tac & 0b11 {
            0 => 9,
            1 => 3,
            2 => 5,
            _ => 7,
        }
    }

    /// The bit whose falling edge (ANDed with timer-enable) clocks TIMA.
    fn mux(&self) -> bool {
        (self.tac & 0b100 != 0) && ((self.counter >> Self::tac_bit(self.tac)) & 1 != 0)
    }

    fn set_counter(&mut self, value: u16) -> bool {
        let before = self.mux();
        self.counter = value;
        // A 1->0 transition of the mux output clocks TIMA once.
        before && !self.mux()
    }

    fn tima_tick(&mut self) {
        let (v, overflow) = self.tima.overflowing_add(1);
        self.tima = v;
        if overflow {
            self.reload = Reload::Overflowed { dots: 4 };
        }
    }
}

impl Chip for Timer {
    fn advance(&mut self, _base_dot: bool) -> Ticked {
        let mut ticked = Ticked::default();

        match self.reload {
            Reload::Overflowed { dots } => {
                let dots = dots - 1;
                if dots == 0 {
                    self.tima = self.tma;
                    ticked.irq |= Interrupts::TIMER;
                    self.reload = Reload::Reloading { dots: 4 };
                } else {
                    self.reload = Reload::Overflowed { dots };
                }
            }
            Reload::Reloading { dots } => {
                self.tima = self.tma;
                let dots = dots - 1;
                self.reload = if dots == 0 {
                    Reload::None
                } else {
                    Reload::Reloading { dots }
                };
            }
            Reload::None => {}
        }

        if self.set_counter(self.counter.wrapping_add(1)) {
            self.tima_tick();
        }
        ticked
    }

    fn bus(&mut self, pins: &mut Pins) -> Ticked {
        match pins.address {
            io::DIV if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = (self.counter >> 8) as u8,
                // Writing DIV clears the whole counter; the drop can clock TIMA.
                BusDir::Write => {
                    if self.set_counter(0) {
                        self.tima_tick();
                    }
                }
                BusDir::Idle => {}
            },
            io::TIMA if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = self.tima,
                BusDir::Write => match self.reload {
                    // Cycle A: Write cancels the overflow entirely
                    Reload::Overflowed { .. } => {
                        self.reload = Reload::None;
                        self.tima = pins.data;
                    }
                    // Cycle B: Write is ignored
                    Reload::Reloading { .. } => {}
                    Reload::None => self.tima = pins.data,
                },
                BusDir::Idle => {}
            },
            io::TMA if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = self.tma,
                BusDir::Write => {
                    self.tma = pins.data;
                    // Cycle B holds TIMA's load line from TMA, so a TMA write
                    // this cycle lands in TIMA on the same cycle.
                    if matches!(self.reload, Reload::Reloading { .. }) {
                        self.tima = pins.data;
                    }
                }
                BusDir::Idle => {}
            },
            io::TAC if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = 0xF8 | self.tac,
                BusDir::Write => {
                    let before = self.mux();
                    self.tac = pins.data & 0x07;
                    if before && !self.mux() {
                        self.tima_tick();
                    }
                }
                BusDir::Idle => {}
            },
            _ => {}
        }
        Ticked::default()
    }
}

impl Timer {
    /// DIV register value, for the APU frame sequencer.
    pub fn div(&self) -> u8 {
        (self.counter >> 8) as u8
    }
}
