use bitflags::bitflags;
use crate::memory::Memory;

pub struct SC1 {
    pace: u8,
    negative_direction: bool,
    step: u8,
    duty_cycle: DutyCycle,
    duty_length_timer: u8,
    volume: u8,
    positive_envelope: bool,
    sweep_pace: u8,
    period: u16,
    trigger: bool,
    length_enabled: bool
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct DutyCycle: u8 {
        const EIGHTH = 0b0000_0000;
        const QUARTER = 0b0000_0001;
        const HALF = 0b0000_00010;
        const THREE_QUARTERS = 0b0000_0011;
    }
}

impl SC1 {
    pub fn new() -> Self {
        Self {
            pace: 0,
            negative_direction: false,
            step: 0,
            duty_cycle: DutyCycle::QUARTER,
            duty_length_timer: 0,
            volume: 0,
            positive_envelope: false,
            sweep_pace: 0,
            period: 0,
            trigger: false,
            length_enabled: false,
        }
    }
}

impl Memory for SC1 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR10: Sweep
            0xFF10 => (self.pace & 0b0000_0111) << 4 | (self.negative_direction as u8) << 3 | (self.step & 0b0000_0111),
            // NR11: Length Timer & Duty Cycle
            0xFF11 => (self.duty_cycle.bits()) << 6,
            // NR12: Volume & Envelope
            0xFF12 => (self.volume & 0b0000_1111) << 4 | (self.positive_envelope as u8) << 3 | (self.sweep_pace & 0b0000_0111),
            // NR13: Period Low
            0xFF13 => 0x00,
            // NR14: Period High & Control
            0xFF14 => (self.length_enabled as u8) << 6,
            _ => panic!("Read to unsupported SC1 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            _ => panic!("Write to unsupported SC1 address ({:#06x})!", a),
        }
    }
}