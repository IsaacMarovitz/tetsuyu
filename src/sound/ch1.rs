use crate::components::memory::Memory;
use crate::sound::apu::DutyCycle;
use crate::sound::length_counter::LengthCounter;
use crate::sound::volume_envelope::VolumeEnvelope;

pub struct CH1 {
    pub dac_enabled: bool,
    sweep_pace: u8,
    negative_direction: bool,
    sweep_step: u8,
    pub duty_cycle: DutyCycle,
    pub period: u16,
    shadow_frequency: u16,
    sweep_enabled: bool,
    sweep_counter: u8,
    frequency_timer: u16,
    pub sample_index: u8,
    pub length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope,
}

impl CH1 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            sweep_pace: 0,
            negative_direction: false,
            sweep_step: 0,
            duty_cycle: DutyCycle::EIGHTH,
            period: 0,
            shadow_frequency: 0,
            sweep_enabled: false,
            sweep_counter: 0,
            frequency_timer: 0,
            sample_index: 0,
            length_counter: LengthCounter::new(),
            volume_envelope: VolumeEnvelope::new(),
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.sweep_pace = 0;
        self.negative_direction = false;
        self.sweep_step = 0;
        self.duty_cycle = DutyCycle::EIGHTH;
        self.period = 0;
        self.shadow_frequency = 0;
        self.sweep_enabled = false;
        self.sweep_counter = 0;
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

    pub fn tick_sweep(&mut self) {
        if !self.sweep_enabled {
            return;
        }

        if self.sweep_counter > 0 {
            self.sweep_counter -= 1;
        }

        if self.sweep_counter == 0 {
            // Reload counter
            self.sweep_counter = if self.sweep_pace > 0 {
                self.sweep_pace
            } else {
                8 // Treat 0 as 8
            };

            if self.sweep_pace > 0 {
                let new_freq = self.calculate_sweep_frequency();

                // Overflow check
                if new_freq <= 0x7FF && self.sweep_step > 0 {
                    self.shadow_frequency = new_freq;
                    self.period = new_freq;

                    // Perform overflow check again
                    let _ = self.calculate_sweep_frequency();
                }
            }
        }
    }

    fn calculate_sweep_frequency(&mut self) -> u16 {
        let offset = self.shadow_frequency >> self.sweep_step;

        let new_freq = if self.negative_direction {
            self.shadow_frequency.saturating_sub(offset)
        } else {
            self.shadow_frequency + offset
        };

        // If overflow, disable channel (APU will handle this)
        if new_freq > 0x7FF {
            self.dac_enabled = false;
        }

        new_freq
    }

    pub fn trigger(&mut self) {
        // Reset frequency timer
        self.frequency_timer = (2048 - self.period) * 4;

        // Reset envelope
        self.volume_envelope.reload();

        // Reset length if zero
        self.length_counter.reload_if_zero(64);

        // Initialize sweep
        self.shadow_frequency = self.period;
        self.sweep_counter = if self.sweep_pace > 0 {
            self.sweep_pace
        } else {
            8
        };
        self.sweep_enabled = self.sweep_pace > 0 || self.sweep_step > 0;

        // If sweep shift is non-zero, do frequency calculation
        if self.sweep_step > 0 {
            let _ = self.calculate_sweep_frequency();
        }
    }
}

impl Memory for CH1 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR10: Sweep
            0xFF10 => {
                (self.sweep_pace & 0b0000_0111) << 4
                    | (self.negative_direction as u8) << 3
                    | (self.sweep_step & 0b0000_0111)
                    | 0x80
            }
            // NR11: Length Timer & Duty Cycle
            0xFF11 => (self.duty_cycle.bits()) << 6 | 0x3F,
            // NR12: Volume & Envelope
            0xFF12 => {
                (self.volume_envelope.volume as u8 & 0b0000_1111) << 4
                    | (self.volume_envelope.positive as u8) << 3
                    | (self.volume_envelope.period as u8 & 0b0000_0111)
            }
            // NR13: Period Low
            0xFF13 => 0xFF,
            // NR14: Period High & Control
            0xFF14 => (self.length_counter.enabled as u8) << 6 | 0xBF,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR10: Sweep
            0xFF10 => {
                self.sweep_pace = (v & 0b0111_0000) >> 4;
                self.negative_direction = ((v & 0b0000_1000) >> 3) != 0;
                self.sweep_step = v & 0b0000_0111;
            }
            // NR11: Length Timer & Duty Cycle
            0xFF11 => {
                self.duty_cycle = DutyCycle::from_bits_truncate(v >> 6);
                self.length_counter.load((v & 0x3F) as u16, 64);
            }
            // NR12: Volume & Envelope
            0xFF12 => {
                let initial_vol = ((v & 0b1111_0000) >> 4) as f32;
                self.volume_envelope.set_initial_volume(initial_vol);
                self.volume_envelope.positive = ((v & 0b0000_1000) >> 3) != 0;
                self.volume_envelope.period = (v & 0b0000_0111) as u16;

                // DAC is enabled if any of bits 3-7 are set
                self.dac_enabled = (v & 0xF8) != 0;
            }
            // NR13: Period Low
            0xFF13 => {
                self.period = (self.period & 0xFF00) | (v as u16);
            }
            // NR14: Period High & Control
            0xFF14 => {
                let trigger = (v & 0x80) != 0;
                self.length_counter.enabled = (v & 0x40) != 0;
                self.period = (self.period & 0x00FF) | (((v & 0x07) as u16) << 8);

                if trigger {
                    self.trigger();
                }
            }
            _ => panic!("Write to unsupported CH1 address ({:#06x})!", a),
        }
    }
}
