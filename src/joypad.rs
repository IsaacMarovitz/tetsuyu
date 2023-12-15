use bitflags::bitflags;
use crate::memory::Memory;
use crate::mmu::Interrupts;

bitflags! {
    #[derive(Copy, Clone)]
    pub struct JoypadButton: u8 {
        const RIGHT = 0b0000_0001;
        const LEFT = 0b0000_0010;
        const UP = 0b0000_0100;
        const DOWN = 0b0000_1000;
        const A = 0b0001_0000;
        const B = 0b0010_0000;
        const SELECT = 0b0100_0000;
        const START = 0b1000_0000;
    }
}

pub struct Joypad {
    matrix: u8,
    select: u8,
    pub interrupts: Interrupts
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            matrix: 0xFF,
            select: 0x00,
            interrupts: Interrupts::empty()
        }
    }

    pub fn down(&mut self, button: JoypadButton) {
        self.matrix &= !button.bits();
        self.interrupts |= Interrupts::JOYPAD;
    }

    pub fn up(&mut self, button: JoypadButton) {
        self.matrix |= button.bits();
    }
}

impl Memory for Joypad {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF00 => {
                if (self.select & 0b0001_0000) == 0x00 {
                    return self.select | (self.matrix & 0x0F);
                }
                if (self.select & 0b0010_0000) == 0x00 {
                    return self.select | (self.matrix >> 4);
                }
                self.select
            }
            _ => panic!("Read to unsupported Joypad address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF00 => self.select = v,
            _ => panic!("Write to unsupported Joypad address ({:#06x})!", a),
        }
    }
}