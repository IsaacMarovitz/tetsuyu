use crate::components::memory::Memory;
use crate::sound::apu::DutyCycle;

pub struct CH2 {
    pub dac_enabled: bool,
    pub duty_cycle: DutyCycle,
    length_timer: u8,
    pub volume: u8,
    positive_envelope: bool,
    envelope_pace: u8,
    pub period: u16,
    pub trigger: bool,
    length_enabled: bool,
    length_cycle_count: u32,
}

impl CH2 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            duty_cycle: DutyCycle::QUARTER,
            length_timer: 0,
            volume: 0,
            positive_envelope: false,
            envelope_pace: 0,
            period: 0,
            trigger: false,
            length_enabled: false,
            length_cycle_count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.duty_cycle = DutyCycle::QUARTER;
        self.length_timer = 0;
        self.volume = 0;
        self.positive_envelope = false;
        self.envelope_pace = 0;
        self.period = 0;
        self.trigger = false;
        self.length_enabled = false;
    }

    pub fn cycle(&mut self) {
        if self.length_enabled {
            self.length_cycle_count += 1;

            if self.length_cycle_count >= 2 {
                self.length_cycle_count = 0;

                if self.length_timer >= 64 {
                    self.dac_enabled = false;
                    self.length_enabled = false;
                } else {
                    self.length_timer += 1;
                }
            }
        }
    }
}

impl Memory for CH2 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR21: Length Timer & Duty Cycle
            0xFF16 => (self.duty_cycle.bits()) << 6 | 0x3F,
            // NR22: Volume & Envelope
            0xFF17 => {
                (self.volume & 0b0000_1111) << 4
                    | (self.positive_envelope as u8) << 3
                    | (self.envelope_pace & 0b0000_0111)
            }
            // NR23: Period Low
            0xFF18 => 0xFF,
            // NR24: Period High & Control
            0xFF19 => (self.length_enabled as u8) << 6 | 0xBF,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR21: Length Timer & Duty Cycle
            0xFF16 => {
                self.duty_cycle = DutyCycle::from_bits_truncate(v >> 6);
                self.length_timer = v & 0b0011_1111;
            }
            // NR22: Volume & Envelope
            0xFF17 => {
                self.volume = (v & 0b1111_0000) >> 4;
                self.positive_envelope = ((v & 0b0000_1000) >> 3) != 0;
                self.envelope_pace = v & 0b0000_0111;

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
                self.trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period &= 0b0000_0000_1111_1111;
                self.period |= ((v & 0b0000_0111) as u16) << 8;
            }
            _ => panic!("Write to unsupported SC2 address ({:#06x})!", a),
        }
    }
}
