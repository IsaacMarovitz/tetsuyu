use crate::components::memory::Memory;
use crate::sound::apu::DutyCycle;
use crate::sound::length_counter::LengthCounter;
use crate::sound::volume_envelope::VolumeEnvelope;

pub struct CH2 {
    pub dac_enabled: bool,
    pub duty_cycle: DutyCycle,
    pub period: u16,
    length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope
}

impl CH2 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            duty_cycle: DutyCycle::EIGHTH,
            period: 0,
            length_counter: LengthCounter::new(),
            volume_envelope: VolumeEnvelope::new()
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.duty_cycle = DutyCycle::EIGHTH;
        self.period = 0;
        self.length_counter.clear();
        self.volume_envelope.clear();
    }

    pub fn cycle(&mut self) {
        self.length_counter.cycle();
        self.volume_envelope.cycle();
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
            // NR21: Length Timer & Duty Cycle
            0xFF15 => {}
            0xFF16 => {
                self.duty_cycle = DutyCycle::from_bits_truncate(v >> 6);
                self.length_counter.counter = (v & 0b0011_1111) as u16;
            }
            // NR22: Volume & Envelope
            0xFF17 => {
                self.volume_envelope.volume = ((v & 0b1111_0000) >> 4) as f32;
                self.volume_envelope.positive = ((v & 0b0000_1000) >> 3) != 0;
                self.volume_envelope.period = (v & 0b0000_0111) as u16;

                if self.read(0xFF17) & 0xF8 != 0 {
                    self.dac_enabled = true;
                }
            }
            // NR23: Period Low
            0xFF18 => {
                self.period &= !0xFF;
                self.period |= v as u16;
            }
            // NR24: Period High & Control
            0xFF19 => {
                self.length_counter.trigger= ((v & 0b1000_0000) >> 7) != 0;
                self.length_counter.enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period &= 0b0000_0000_1111_1111;
                self.period |= ((v & 0b0000_0111) as u16) << 8;

                if self.length_counter.trigger {
                    self.length_counter.reload(1 << 6);
                }
            }
            _ => panic!("Write to unsupported SC2 address ({:#06x})!", a),
        }
    }
}
