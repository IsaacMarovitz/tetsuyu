use crate::components::memory::Memory;
use crate::sound::apu::DutyCycle;
use crate::sound::length_counter::LengthCounter;
use crate::sound::volume_envelope::VolumeEnvelope;

pub struct CH2 {
    pub dac_enabled: bool,
    pub duty_cycle: DutyCycle,
    pub period: u16,
    frequency_timer: u16,
    pub sample_index: u8,
    pub length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope,
}

impl CH2 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            duty_cycle: DutyCycle::EIGHTH,
            period: 0,
            frequency_timer: 0,
            sample_index: 0,
            length_counter: LengthCounter::new(),
            volume_envelope: VolumeEnvelope::new(),
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.duty_cycle = DutyCycle::EIGHTH;
        self.period = 0;
        self.frequency_timer = 0;
        self.sample_index = 0;
        self.length_counter.clear();
        self.volume_envelope.clear();
    }

    pub fn tick_frequency(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        } else {
            // Reload timer: (2048 - period) * 4
            self.frequency_timer = (2048 - self.period) * 4;

            // Advance to next sample in duty cycle
            self.sample_index = (self.sample_index + 1) & 0x07;
        }
    }

    pub fn trigger(&mut self) {
        // Reset frequency timer
        self.frequency_timer = (2048 - self.period) * 4;

        // Reset envelope
        self.volume_envelope.reload();

        // Reset length if zero
        self.length_counter.reload_if_zero(64);
    }
}

impl Memory for CH2 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR21: Length Timer & Duty Cycle
            0xFF16 => (self.duty_cycle.bits()) << 6 | 0x3F,
            // NR22: Volume & Envelope
            0xFF17 => {
                (self.volume_envelope.volume as u8 & 0b0000_1111) << 4
                    | (self.volume_envelope.positive as u8) << 3
                    | (self.volume_envelope.period as u8 & 0b0000_0111)
            }
            // NR23: Period Low
            0xFF18 => 0xFF,
            // NR24: Period High & Control
            0xFF19 => (self.length_counter.enabled as u8) << 6 | 0xBF,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR20: Unused
            0xFF15 => {}
            // NR21: Length Timer & Duty Cycle
            0xFF16 => {
                self.duty_cycle = DutyCycle::from_bits_truncate(v >> 6);
                self.length_counter.load((v & 0x3F) as u16, 64);
            }
            // NR22: Volume & Envelope
            0xFF17 => {
                let initial_vol = ((v & 0b1111_0000) >> 4) as f32;
                self.volume_envelope.set_initial_volume(initial_vol);
                self.volume_envelope.positive = ((v & 0b0000_1000) >> 3) != 0;
                self.volume_envelope.period = (v & 0b0000_0111) as u16;

                // DAC is enabled if any of bits 3-7 are set
                self.dac_enabled = (v & 0xF8) != 0;
            }
            // NR23: Period Low
            0xFF18 => {
                self.period = (self.period & 0xFF00) | (v as u16);
            }
            // NR24: Period High & Control
            0xFF19 => {
                let trigger = (v & 0x80) != 0;
                self.length_counter.enabled = (v & 0x40) != 0;
                self.period = (self.period & 0x00FF) | (((v & 0x07) as u16) << 8);

                if trigger {
                    self.trigger();
                }
            }
            _ => panic!("Write to unsupported CH2 address ({:#06x})!", a),
        }
    }
}
