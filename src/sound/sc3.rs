use bitflags::bitflags;
use crate::memory::Memory;

pub struct SC3 {
    dac_enabled: bool,
    length_timer: u8,
    output_level: OutputLevel,
    period: u16,
    trigger: bool,
    length_enabled: bool
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct OutputLevel: u8 {
        const MUTE = 0b0000_0000;
        const MAX = 0b0010_0000;
        const HALF = 0b0100_0000;
        const QUARTER = 0b0110_0000;
    }
}

impl SC3 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            length_timer: 0,
            output_level: OutputLevel::MUTE,
            period: 0,
            trigger: false,
            length_enabled: false,
        }
    }
}

impl Memory for SC3 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR30: DAC Enable
            0xFF1A => (self.dac_enabled as u8) << 7,
            // NR31: Length Timer
            0xFF1B => 0x00,
            // NR32: Output Level
            0xFF1C => self.output_level.bits(),
            // NR33: Period Low
            0xFF1D => 0x00,
            // NR34: Period High & Control
            0xFF1E => (self.length_enabled as u8) << 6,
            _ => panic!("Read to unsupported SC3 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF1A => self.dac_enabled = ((v & 0b1000_0000) >> 7) != 0,
            0xFF1B => self.length_timer = v,
            0xFF1C => self.output_level = OutputLevel::from_bits_truncate(v),
            0xFF1D => {
                self.period &= !0xFF;
                self.period |= v as u16;
            },
            0xFF1E => {
                self.trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period &= 0b0000_0000_1111_1111;
                self.period |= ((v & 0b0000_0111) as u16) << 8;
            },
            _ => panic!("Write to unsupported SC3 address ({:#06x})!", a),
        }
    }
}