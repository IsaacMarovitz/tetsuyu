/// A down-counting timer shared by the tone and wave channels. It advances one
/// T-cycle per `tick` and, when the period elapses, reloads with the caller's
/// current period and signals the channel to step. Passing the period on each
/// tick lets a mid-period frequency change take effect at the next reload, as
/// on hardware. The reload is exact: a period of N signals every N ticks, with
/// no off-by-one.
pub struct PeriodTimer {
    counter: u16,
}

impl PeriodTimer {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Load the counter directly. Used on trigger, where a channel may fold its
    /// own startup delay into the first period.
    pub fn set(&mut self, ticks: u16) {
        self.counter = ticks;
    }

    /// Advance one T-cycle, reloading with `period` and returning true when the
    /// period elapses.
    pub fn tick(&mut self, period: u16) -> bool {
        if self.counter > 0 {
            self.counter -= 1;
        }
        if self.counter == 0 {
            self.counter = period;
            true
        } else {
            false
        }
    }

    /// T-cycles left before the next reload.
    pub fn remaining(&self) -> u16 {
        self.counter
    }
}
