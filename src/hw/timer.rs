use super::bus::{BusDir, Chip, Pins, Ticked};
use super::interrupt::Interrupts;

/// State of the TIMA overflow → reload sequence. When TIMA overflows from an
/// increment it does not reload from TMA immediately: for the M-cycle after the
/// overflow TIMA reads `$00` (cycle A of the Pandocs description), and only on
/// the following M-cycle (cycle B) is TMA copied into TIMA and the interrupt
/// requested. CPU writes interact specially with each phase, so the phase is
/// modelled explicitly rather than as a bare countdown.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reload {
    /// No overflow in flight.
    None,
    /// Cycle A: TIMA just overflowed and reads `$00`. `dots` counts down the
    /// remaining T-cycles of this M-cycle; at the boundary we advance to
    /// `Pending`. A TIMA write during this phase cancels the whole sequence
    /// (the overflow "didn't happen").
    Overflowed { dots: u8 },
    /// Cycle B: TMA is being copied into TIMA and the interrupt raised. Held
    /// for one M-cycle. A TIMA write here is ignored (TMA wins); a TMA write
    /// here lands in TIMA too.
    Reloading { dots: u8 },
}

/// DIV/TIMA timer as a peer chip. The internal 16-bit counter advances one
/// step per dot (CPU-clock domain). DIV is the high byte; TIMA is clocked on
/// the falling edge of a selected counter bit, which is what makes the DIV /
/// TAC write glitches fall out instead of being special-cased.
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

    /// Clock TIMA once. On overflow TIMA wraps to `$00` and enters the
    /// `Overflowed` phase: the TMA reload + interrupt are deferred one M-cycle
    /// (cycle B) rather than happening now. A manual TIMA write never triggers
    /// this — only an increment can.
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
        // CPU-clock domain: advance one step every call regardless of base_dot
        // (the timer runs at the CPU rate, so double-speed halves nothing here).
        let mut ticked = Ticked::default();

        // Step the overflow → reload sequence. Each phase lasts one M-cycle
        // (4 dots); the transitions happen at the M-cycle boundary.
        match self.reload {
            Reload::Overflowed { dots } => {
                let dots = dots - 1;
                if dots == 0 {
                    // Enter cycle B: copy TMA into TIMA and raise the interrupt.
                    // The load is held for the whole M-cycle (Reloading), so a
                    // TMA write landing this cycle is reflected in TIMA too.
                    self.tima = self.tma;
                    ticked.irq |= Interrupts::TIMER;
                    self.reload = Reload::Reloading { dots: 4 };
                } else {
                    self.reload = Reload::Overflowed { dots };
                }
            }
            Reload::Reloading { dots } => {
                // TIMA constantly copies TMA through cycle B; re-apply it each
                // dot so a mid-cycle TMA write is picked up, and a TIMA write
                // during this cycle is overwritten.
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
                BusDir::Write => match self.reload {
                    // Cycle A: the write cancels the overflow entirely — the
                    // reload and interrupt never happen and the written value
                    // sticks.
                    Reload::Overflowed { .. } => {
                        self.reload = Reload::None;
                        self.tima = pins.data;
                    }
                    // Cycle B: the write is ignored; TIMA is being driven from
                    // TMA for the whole cycle and ends up equal to it.
                    Reload::Reloading { .. } => {}
                    Reload::None => self.tima = pins.data,
                },
                BusDir::Idle => {}
            },
            0xFF06 if pins.selected(true) => match pins.dir {
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
