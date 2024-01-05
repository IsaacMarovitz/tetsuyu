use crate::memory::Memory;

pub struct SC4 {
    pub dac_enabled: bool,
    length_timer: u8,
    pub volume: u8,
    positive_envelope: bool,
    envelope_pace: u8,
    clock: u8,
    // False = 15-bit, True = 7-bit
    lfsr_width: bool,
    clock_divider: u8,
    pub trigger: bool,
    length_enabled: bool
}

impl SC4 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            length_timer: 0,
            volume: 0,
            positive_envelope: false,
            envelope_pace: 0,
            clock: 0,
            lfsr_width: false,
            clock_divider: 0,
            trigger: false,
            length_enabled: false,
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.length_timer = 0;
        self.volume = 0;
        self.positive_envelope = false;
        self.envelope_pace = 0;
        self.clock = 0;
        self.lfsr_width = false;
        self.clock_divider = 0;
        self.trigger = false;
        self.length_enabled = false;
    }

    pub fn cycle(&mut self, cycles: u32) {

    }
}

impl Memory for SC4 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR41: Length Timer
            0xFF20 => 0xFF,
            // NR42: Volume & Envelope
            0xFF21 => (self.volume & 0b0000_1111) << 4 | (self.positive_envelope as u8) << 3 | (self.envelope_pace & 0b0000_0111),
            // NR43: Frequency & Randomness
            0xFF22 => (self.clock & 0b0000_1111 << 4) | (self.lfsr_width as u8) << 3 | (self.clock_divider & 0b0000_0111),
            // NR44: Control
            0xFF23 => (self.length_enabled as u8) << 6 | 0xBF,
            _ => 0xFF
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR41: Length Timer
            0xFF20 => self.length_timer = v & 0b0011_1111,
            // NR42: Volume & Envelope
            0xFF21 => {
                self.volume = (v & 0b1111_0000) >> 4;
                self.positive_envelope = ((v & 0b0000_1000) >> 3) != 0;
                self.envelope_pace = v & 0b0000_0111;

                if self.read(0xFF21) & 0xF8 != 0 {
                    self.dac_enabled = true;
                }
            },
            // NR43: Frequency & Randomness
            0xFF22 => {
                self.clock = (v & 0b1111_0000) >> 4;
                self.lfsr_width = ((v & 0b0000_1000) >> 3) != 0;
                self.clock_divider = v & 0b0000_0111;
            },
            // NR44: Control
            0xFF23 => {
                self.trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_enabled = ((v & 0b0100_0000) >> 6) != 0;
            },
            _ => panic!("Write to unsupported SC4 address ({:#06x})!", a),
        }
    }
}