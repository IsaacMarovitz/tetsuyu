use crate::components::memory::Memory;
use crate::sound::length_counter::LengthCounter;
use crate::sound::volume_envelope::VolumeEnvelope;

pub struct CH4 {
    pub dac_enabled: bool,
    clock_shift: u8,
    lfsr_width: bool,
    divisor_code: u8,
    pub lfsr: u16,
    pub final_volume: f32,
    pub length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope,
}

impl CH4 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            clock_shift: 0,
            lfsr_width: false,
            divisor_code: 0,
            lfsr: 0x0000,
            final_volume: 0.0,
            length_counter: LengthCounter::new(),
            volume_envelope: VolumeEnvelope::new(),
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.clock_shift = 0;
        self.lfsr_width = false;
        self.divisor_code = 0;
        self.lfsr = 0x0000;
        self.final_volume = 0.0;
        self.length_counter.clear();
        self.volume_envelope.clear();
    }

    fn get_divisor(&self) -> u32 {
        match self.divisor_code {
            0 => 8,
            n => (n as u32) * 16,
        }
    }

    pub fn tick_lfsr(&mut self) {
        self.final_volume = self.volume_envelope.volume;
    }

    pub fn trigger(&mut self) {
        self.lfsr = 0x7FFF;
        self.volume_envelope.reload();
        self.length_counter.reload_if_zero(64);
        self.final_volume = self.volume_envelope.volume;
    }

    pub fn get_frequency(&self) -> f32 {
        let divisor = self.get_divisor() as f32;

        // Game Boy noise frequency formula:
        // Base clock is 524288 Hz (4.194304 MHz / 8)
        // Frequency = 524288 / divisor / 2^shift
        let base_clock = 524288.0;
        let shift_divisor = (1 << self.clock_shift) as f32;

        let frequency = base_clock / divisor / shift_divisor;

        // Clamp to prevent aliasing and ensure audible range
        frequency.max(50.0).min(22000.0)
    }

    pub fn is_width_7bit(&self) -> bool {
        self.lfsr_width
    }
}

impl Memory for CH4 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR41: Length Timer
            0xFF20 => 0xFF,
            // NR42: Volume & Envelope
            0xFF21 => {
                (self.volume_envelope.volume as u8 & 0b0000_1111) << 4
                    | (self.volume_envelope.positive as u8) << 3
                    | (self.volume_envelope.period as u8 & 0b0000_0111)
            }
            // NR43: Frequency & Randomness
            0xFF22 => {
                ((self.clock_shift & 0b0000_1111) << 4)
                    | ((self.lfsr_width as u8) << 3)
                    | (self.divisor_code & 0b0000_0111)
            }
            // NR44: Control
            0xFF23 => (self.length_counter.enabled as u8) << 6 | 0xBF,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF1F => {}
            // NR41: Length Timer
            0xFF20 => {
                self.length_counter.load((v & 0x3F) as u16, 64);
            }
            // NR42: Volume & Envelope
            0xFF21 => {
                let initial_vol = ((v & 0b1111_0000) >> 4) as f32;
                self.volume_envelope.set_initial_volume(initial_vol);
                self.volume_envelope.positive = ((v & 0b0000_1000) >> 3) != 0;
                self.volume_envelope.period = (v & 0b0000_0111) as u16;

                self.dac_enabled = (v & 0xF8) != 0;
            }
            // NR43: Frequency & Randomness
            0xFF22 => {
                self.clock_shift = (v & 0b1111_0000) >> 4;
                self.lfsr_width = ((v & 0b0000_1000) >> 3) != 0;
                self.divisor_code = v & 0b0000_0111;
            }
            // NR44: Control
            0xFF23 => {
                let trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_counter.enabled = ((v & 0b0100_0000) >> 6) != 0;

                if trigger {
                    self.trigger();
                }
            }
            _ => panic!("Write to unsupported CH4 address ({:#06x})!", a),
        }
    }
}
