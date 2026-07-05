use super::bus::{BusDir, Chip, Pins, Ticked};
use super::interrupt::Interrupts;

/// DIV/TIMA timer as a peer chip. The internal 16-bit counter advances one
/// step per dot (CPU-clock domain). DIV is the high byte; TIMA is clocked on
/// the falling edge of a selected counter bit, which is what makes the DIV /
/// TAC write glitches fall out instead of being special-cased.
pub struct Timer {
    counter: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    /// T-cycles remaining until a TIMA overflow reloads TMA and raises the
    /// interrupt. 0 means no reload pending. Hardware delays this by one
    /// M-cycle: TIMA reads 0x00 during the window.
    reload: u8,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            reload: 0,
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

    /// Clock TIMA once. On overflow TIMA wraps to 0x00 and the TMA reload +
    /// interrupt are scheduled one M-cycle later, rather than happening now.
    fn tima_tick(&mut self) {
        let (v, overflow) = self.tima.overflowing_add(1);
        self.tima = v;
        if overflow {
            self.reload = 4;
        }
    }
}

impl Chip for Timer {
    fn advance(&mut self, _base_dot: bool) -> Ticked {
        // CPU-clock domain: advance one step every call regardless of base_dot
        // (the timer runs at the CPU rate, so double-speed halves nothing here).
        let mut ticked = Ticked::default();

        // A pending overflow reloads TMA and raises the interrupt after its
        // 4-T-cycle delay.
        if self.reload > 0 {
            self.reload -= 1;
            if self.reload == 0 {
                self.tima = self.tma;
                ticked.irq |= Interrupts::TIMER;
            }
        }

        if self.set_counter(self.counter.wrapping_add(1)) {
            self.tima_tick();
        }
        ticked
    }

    fn bus(&mut self, pins: &mut Pins) -> Ticked {
        match pins.address {
            0xFF04 if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = (self.counter >> 8) as u8,
                // Writing DIV clears the whole counter; the drop can clock TIMA.
                BusDir::Write => {
                    if self.set_counter(0) {
                        self.tima_tick();
                    }
                }
                BusDir::Idle => {}
            },
            0xFF05 if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = self.tima,
                // Writing TIMA during the reload window cancels the reload.
                BusDir::Write => {
                    self.reload = 0;
                    self.tima = pins.data;
                }
                BusDir::Idle => {}
            },
            0xFF06 if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = self.tma,
                BusDir::Write => self.tma = pins.data,
                BusDir::Idle => {}
            },
            0xFF07 if pins.selected(true) => match pins.dir {
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
