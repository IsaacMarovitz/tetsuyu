use crate::components::memory::Memory;
use crate::sound::length_counter::LengthCounter;
use bitflags::bitflags;

pub struct CH3 {
    pub dac_enabled: bool,
    pub output_level: OutputLevel,
    pub period: u16,
    wave_ram: [u8; 16],
    frequency_timer: u16,
    pub sample_index: u8,
    pub length_counter: LengthCounter,
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
            output_level: OutputLevel::MUTE,
            period: 0,
            wave_ram: [0; 16],
            frequency_timer: 0,
            sample_index: 0,
            length_counter: LengthCounter::new(),
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.output_level = OutputLevel::MUTE;
        self.period = 0;
        self.frequency_timer = 0;
        self.sample_index = 0;
        self.length_counter.clear();
    }

    pub fn tick_frequency(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        } else {
            // Reload timer: (2048 - period) * 2
            self.frequency_timer = (2048 - self.period) * 2;

            // Advance to next sample (32 samples total)
            self.sample_index = (self.sample_index + 1) & 0x1F;
        }
    }

    pub fn trigger(&mut self) {
        // Reset frequency timer
        self.frequency_timer = (2048 - self.period) * 2;

        // Reset sample position
        self.sample_index = 0;

        // Reset length if zero
        self.length_counter.reload_if_zero(256);
    }

    pub fn get_volume_shift(&self) -> u8 {
        match self.output_level {
            OutputLevel::MAX => 0,
            OutputLevel::HALF => 1,
            OutputLevel::QUARTER => 2,
            OutputLevel::MUTE => 4,
            _ => 4,
        }
    }

    pub fn get_current_sample(&self) -> u8 {
        let byte_index = (self.sample_index / 2) as usize;
        let byte = self.wave_ram[byte_index];

        // High nibble for even indices, low nibble for odd
        let sample = if self.sample_index & 1 == 0 {
            (byte >> 4) & 0x0F
        } else {
            byte & 0x0F
        };

        // Apply output level shift
        sample >> self.get_volume_shift()
    }

    pub fn get_current_sample_f32(&self) -> f32 {
        let sample = self.get_current_sample();
        // Normalize to -1.0 to 1.0
        // After shifting, max value depends on shift amount:
        // shift 0: 0-15, shift 1: 0-7, shift 2: 0-3, shift 4: 0
        let max_value = match self.get_volume_shift() {
            0 => 15.0,
            1 => 7.0,
            2 => 3.0,
            _ => 0.0,
        };

        if max_value > 0.0 {
            ((sample as f32 / max_value) * 2.0) - 1.0
        } else {
            0.0
        }
    }

    pub fn wave_as_f32(&self) -> [f32; 32] {
        const U4_MAX: f32 = 0b1111 as f32;
        let mut wave: [f32; 32] = [0f32; 32];

        for i in 0..self.wave_ram.len() {
            wave[i * 2] = ((((self.wave_ram[i] & 0b1111_0000) >> 4) as f32 / U4_MAX) * 2.0) - 1.0;
            wave[(i * 2) + 1] = (((self.wave_ram[i] & 0b0000_1111) as f32 / U4_MAX) * 2.0) - 1.0;
        }

        wave
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
            0xFF1E => (self.length_counter.enabled as u8) << 6 | 0xBF,
            0xFF30..=0xFF3F => {
                if !self.dac_enabled {
                    self.wave_ram[a as usize - 0xFF30]
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR30: DAC Enable
            0xFF1A => {
                self.dac_enabled = ((v & 0b1000_0000) >> 7) != 0;
            }
            // NR31: Length Timer
            0xFF1B => {
                self.length_counter.load(v as u16, 256);
            }
            // NR32: Output Level
            0xFF1C => {
                self.output_level = OutputLevel::from_bits_truncate(v & 0b0110_0000);
            }
            // NR33: Period Low
            0xFF1D => {
                self.period = (self.period & 0xFF00) | (v as u16);
            }
            // NR34: Period High & Control
            0xFF1E => {
                let trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_counter.enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period = (self.period & 0x00FF) | (((v & 0x07) as u16) << 8);

                if trigger {
                    self.trigger();
                }
            }
            0xFF30..=0xFF3F => {
                if !self.dac_enabled {
                    self.wave_ram[a as usize - 0xFF30] = v;
                }
            }
            _ => panic!("Write to unsupported CH3 address ({:#06x})!", a),
        }
    }
}
