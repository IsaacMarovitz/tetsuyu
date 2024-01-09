use bitflags::bitflags;
use crate::memory::Memory;

pub struct CH3 {
    pub dac_enabled: bool,
    length_timer: u8,
    pub output_level: OutputLevel,
    pub period: u16,
    pub trigger: bool,
    length_enabled: bool,
    wave_ram: [u8; 16]
}

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct OutputLevel: u8 {
        const MUTE = 0b0000_0000;
        const MAX = 0b0010_0000;
        const HALF = 0b0100_0000;
        const QUARTER = 0b0110_0000;
    }
}

impl CH3 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            length_timer: 0,
            output_level: OutputLevel::MUTE,
            period: 0,
            trigger: false,
            length_enabled: false,
            wave_ram: [0; 16]
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.length_timer = 0;
        self.output_level = OutputLevel::MUTE;
        self.period = 0;
        self.trigger = false;
        self.length_enabled = false;
    }

    pub fn cycle(&mut self) {

    }
}

impl Memory for CH3 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR30: DAC Enable
            0xFF1A => (self.dac_enabled as u8) << 7 | 0x7F,
            // NR31: Length Timer
            0xFF1B => 0xFF,
            // NR32: Output Level
            0xFF1C => self.output_level.bits() | 0x9F,
            // NR33: Period Low
            0xFF1D => 0xFF,
            // NR34: Period High & Control
            0xFF1E => (self.length_enabled as u8) << 6 | 0xBF,
            0xFF30..=0xFF3F => {
                if !self.dac_enabled {
                    self.wave_ram[a as usize - 0xFF30]
                } else {
                    0xFF
                }
            },
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR30: DAC Enable
            0xFF1A => self.dac_enabled = ((v & 0b1000_0000) >> 7) != 0,
            // NR31: Length Timer
            0xFF1B => self.length_timer = v,
            // NR32: Output Level
            0xFF1C => self.output_level = OutputLevel::from_bits_truncate(v),
            // NR33: Period Low
            0xFF1D => {
                self.period &= !0xFF;
                self.period |= v as u16;
            },
            // NR34: Period High & Control
            0xFF1E => {
                self.trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period &= 0b0000_0000_1111_1111;
                self.period |= ((v & 0b0000_0111) as u16) << 8;
            },
            0xFF30..=0xFF3F => {
                if !self.dac_enabled {
                    self.wave_ram[a as usize - 0xFF30] = v;
                }
            },
            _ => panic!("Write to unsupported SC3 address ({:#06x})!", a),
        }
    }
}