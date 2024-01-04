use bitflags::bitflags;
use crate::memory::Memory;

pub struct SC2 {
    duty_cycle: DutyCycle,
    duty_length_timer: u8,
    volume: u8,
    positive_envelope: bool,
    sweep_pace: u8,
    period: u16,
    pub trigger: bool,
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

impl SC2 {
    pub fn new() -> Self {
        Self {
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

impl Memory for SC2 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR21: Length Timer & Duty Cycle
            0xFF16 => (self.duty_cycle.bits()) << 6 | 0x3F,
            // NR22: Volume & Envelope
            0xFF17 => (self.volume & 0b0000_1111) << 4 | (self.positive_envelope as u8) << 3 | (self.sweep_pace & 0b0000_0111),
            // NR23: Period Low
            0xFF18 => 0xFF,
            // NR24: Period High & Control
            0xFF19 => (self.length_enabled as u8) << 6 | 0xBF,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF16 => {
                self.duty_cycle = DutyCycle::from_bits_truncate(v >> 6);
                self.duty_length_timer = v & 0b0011_1111;
            },
            0xFF17 => {
                self.volume = (v & 0b1111_0000) >> 4;
                self.positive_envelope = ((v & 0b0000_1000) >> 3) != 0;
                self.sweep_pace = v & 0b0000_0111;
            },
            0xFF18 => {
                self.period &= !0xFF;
                self.period |= v as u16;
            },
            0xFF19 => {
                self.trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period &= 0b0000_0000_1111_1111;
                self.period |= ((v & 0b0000_0111) as u16) << 8;
            },
            _ => panic!("Write to unsupported SC2 address ({:#06x})!", a),
        }
    }
}