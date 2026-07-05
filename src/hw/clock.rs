/// Owns *time* and nothing else. It does not own or step any chip; its only
/// job is to turn CPU T-cycles into base-clock dots, accounting for the
/// double-speed divider. This is what keeps the ownership graph acyclic — the
/// Clock has no back-references to the motherboard or its chips.
pub struct Clock {
    /// Total base-clock dots elapsed since power-on.
    dots: u64,
    /// Divider phase for double-speed: at 2x the CPU issues T-cycles twice as
    /// fast as the base clock, so only every other CPU tick is a base dot.
    speed_phase: u8,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            dots: 0,
            speed_phase: 0,
        }
    }

    /// Advance one CPU T-cycle. Returns whether a base-clock dot occurs on this
    /// tick — CPU-domain chips (timer, DMA) step every tick; base-domain chips
    /// (PPU, APU) step only when this returns true.
    ///
    /// 1x: every CPU tick is a base dot.
    /// 2x: every second CPU tick is a base dot (CPU runs twice as fast).
    pub fn tick(&mut self, double_speed: bool) -> bool {
        if double_speed {
            self.speed_phase ^= 1;
            if self.speed_phase == 0 {
                self.dots += 1;
                true
            } else {
                false
            }
        } else {
            self.dots += 1;
            true
        }
    }

    pub fn dots(&self) -> u64 {
        self.dots
    }
}
