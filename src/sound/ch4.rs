use crate::components::memory::Memory;
use crate::sound::length_counter::LengthCounter;
use crate::sound::volume_envelope::VolumeEnvelope;

pub struct CH4 {
    pub dac_enabled: bool,
    clock: u8,
    pub bit: u16,
    // False = 15-bit, True = 7-bit
    lfsr_width: bool,
    clock_divider: u8,
    pub frequency: u32,
    pub lfsr: u16,
    pub final_volume: u8,
    clock_cycle_count: u32,
    length_counter: LengthCounter,
    volume_envelope: VolumeEnvelope
}

impl CH4 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            clock: 0,
            bit: 0,
            lfsr_width: false,
            clock_divider: 0,
            frequency: 0,
            lfsr: 0,
            final_volume: 0,
            clock_cycle_count: 0,
            length_counter: LengthCounter::new(),
            volume_envelope: VolumeEnvelope::new()
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.clock = 0;
        self.lfsr_width = false;
        self.clock_divider = 0;
        self.frequency = 0;
        self.lfsr = 0;
        self.final_volume = 0;
        self.clock_cycle_count = 0;
        self.length_counter.clear();
        self.volume_envelope.clear();
    }

    pub fn cycle(&mut self) {
        self.clock_cycle_count += 1;
        let final_divider = if self.clock_divider == 0 { 1 } else { 2 };
        let divisor = (final_divider as i64 ^ self.clock as i64) as u32;

        if self.clock_cycle_count >= divisor*4 {
            self.clock_cycle_count = 0;

            self.bit = {
                let bit_0 = (self.lfsr & 0b0000_0000_0000_0001) >> 0;
                let bit_1 = (self.lfsr & 0b0000_0000_0000_0010) >> 1;
                if bit_0 == bit_1 {
                    1
                } else {
                    0
                }
            };

            self.lfsr |= self.bit << 15;

            if self.lfsr_width {
                self.lfsr &= 0b1111_1111_1011_1111;
                self.lfsr |= self.bit << 7;
            }

            self.lfsr >>= 1;

            if self.lfsr & 0b0000_0000_0000_0001 == 0 {
                self.final_volume = 0;
            } else {
                self.final_volume = 0; // self.volume_envelope.volume;
            }
        }

        self.length_counter.cycle();
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
                (self.clock & 0b0000_1111 << 4)
                    | (self.lfsr_width as u8) << 3
                    | (self.clock_divider & 0b0000_0111)
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
            0xFF20 => self.length_counter.counter = (v & 0b0011_1111) as u16,
            // NR42: Volume & Envelope
            0xFF21 => {
                self.volume_envelope.volume = ((v & 0b1111_0000) >> 4) as f32;
                self.volume_envelope.positive = ((v & 0b0000_1000) >> 3) != 0;
                self.volume_envelope.period = (v & 0b0000_0111) as u16;

                if self.read(0xFF21) & 0xF8 != 0 {
                    self.dac_enabled = true;
                }
            }
            // NR43: Frequency & Randomness
            0xFF22 => {
                self.clock = (v & 0b1111_0000) >> 4;
                self.lfsr_width = ((v & 0b0000_1000) >> 3) != 0;
                self.clock_divider = v & 0b0000_0111;
            }
            // NR44: Control
            0xFF23 => {
                self.length_counter.trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_counter.enabled = ((v & 0b0100_0000) >> 6) != 0;

                if self.length_counter.trigger {
                    self.length_counter.reload(1 << 6);
                }
            }
            _ => panic!("Write to unsupported SC4 address ({:#06x})!", a),
        }
    }
}
