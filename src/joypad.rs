use bitflags::bitflags;
use crate::memory::Memory;
use crate::mmu::Interrupts;

bitflags! {
    #[derive(Copy, Clone)]
    pub struct JoypadButton: u8 {
        const A = 0b0000_0001;
        const B = 0b0000_0010;
        const SELECT = 0b0000_0100;
        const START = 0b0000_1000;
        const RIGHT = 0b0001_0000;
        const LEFT = 0b0010_0000;
        const UP = 0b0100_0000;
        const DOWN = 0b1000_0000;
    }
}

pub struct Joypad {
    matrix: u8,
    select: u8,
    previous_select: u8,
    pub interrupts: Interrupts
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            matrix: 0xFF,
            select: 0x0F,
            previous_select: 0x0F,
            interrupts: Interrupts::empty()
        }
    }

    pub fn down(&mut self, button: JoypadButton) {
        self.matrix &= !button.bits();
        self.update_joypad();
    }

    pub fn up(&mut self, button: JoypadButton) {
        self.matrix |= button.bits();
    }

    pub fn update_joypad(&mut self) {
        let new_select = self.read(0xFF00) & 0x0F;

        if self.previous_select == 0x0F && new_select != 0x0F {
            self.interrupts |= Interrupts::JOYPAD;
        }

        self.previous_select = new_select;
    }
}

impl Memory for Joypad {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF00 => {
                // D-Pad
                if (self.select & 0b0001_0000) == 0x00 {
                    return self.select | (self.matrix >> 4);
                }
                // Buttons
                if (self.select & 0b0010_0000) == 0x00 {
                    return self.select | (self.matrix & 0x0F);
                }
                self.select
            }
            _ => panic!("Read to unsupported Joypad address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF00 => self.select = (v & 0x30),
            _ => panic!("Write to unsupported Joypad address ({:#06x})!", a),
        }

        self.update_joypad();
    }
}