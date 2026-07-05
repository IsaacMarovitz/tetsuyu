use super::bus::{BusDir, Pins};
use crate::components::memory::Memory;
use crate::components::mode::GBMode;
use crate::config::APUConfig;
use crate::sound::apu::APU as CoreApu;

/// APU as a bus peer. Its frame sequencer is clocked by the timer's DIV, so the
/// motherboard advances it each M-cycle with the current DIV value rather than
/// through the generic `Chip::advance` path (which has no DIV to hand it).
pub struct Apu {
    core: CoreApu,
}

impl Apu {
    pub fn new(config: APUConfig, mode: GBMode) -> Self {
        Self {
            core: CoreApu::new(config, mode),
        }
    }

    /// Advance the APU, driven by the DIV-APU edge (falling bit 4/5).
    pub fn advance(&mut self, div: u8, double_speed: bool) {
        self.core.cycle(div, double_speed);
    }

    pub fn bus(&mut self, pins: &mut Pins) {
        if pins.transfer && matches!(pins.address, 0xFF10..=0xFF3F) {
            match pins.dir {
                BusDir::Read => pins.data = self.core.read(pins.address),
                BusDir::Write => self.core.write(pins.address, pins.data),
                BusDir::Idle => {}
            }
        }
    }
}
