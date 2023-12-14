use crate::mmu::Interrupts;

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    pub interrupts: Interrupts,
    enabled: bool,
    step: u32,
    internal_count: u32,
    internal_divider: u32
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0x00,
            tima: 0x00,
            tma: 0x00,
            interrupts: Interrupts::empty(),
            enabled: false,
            step: 256,
            internal_count: 0,
            internal_divider: 0
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        self.internal_divider += cycles;
        while self.internal_divider >= 256 {
            self.div = self.div.wrapping_add(1);
            self.internal_divider -= 256;
        }

        if self.enabled {
            self.internal_count += cycles;

            while self.internal_count >= self.step {
                self.tima = self.tima.wrapping_add(1);
                if self.tima == 0x00 {
                    self.tima = self.tma;
                    self.interrupts |= Interrupts::TIMER;
                }
                self.internal_count -= self.step;
            }
        }
    }

    pub fn read(&self, a: u16) -> u8 {
        match a {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => {
                let mut v = 0xF8;
                v |= if self.enabled { 0b0000_0100 } else { 0x00 };
                v |= match self.step {
                    1024 => 1,
                    16 => 2,
                    64 => 3,
                    _ => panic!("Unknown timer step ({})!", self.step)
                };

                v
            },
            _ => panic!("Read to unsupported timer address ({:#06x})!", a),
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF04 => self.div = 0x00,
            0xFF05 => self.tima = v,
            0xFF06 => self.tma = v,
            0xFF07 => {
                self.enabled = (v & 0b0000_0100) != 0;
                self.step = match v & 0b0000_0011 {
                    0 => 1024,
                    1 => 16,
                    2 => 64,
                    3 => 256,
                    _ => panic!("Unknown timer step ({})!", v)
                }
            },
            _ => panic!("Write to unsupported timer address ({:#06x})!", a),
        }
    }
}