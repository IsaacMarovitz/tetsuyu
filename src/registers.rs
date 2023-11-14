use bitflags::bitflags;
use crate::mode::GBMode;

pub struct Registers {
    pub a: u8,
    f: Flags,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pc: u16,
    sp: u16,
}

bitflags! {
    pub struct Flags: u8 {
        // Carry Flag
        const C = 0b0001_0000;
        // Half-Carry Flag
        const H = 0b0010_0000;
        // Subtract Flag
        const N = 0b0100_0000;
        // Zero Flag
        const Z = 0b1000_0000;
    }
}

impl Registers {
    pub fn get_flag(&self, flag: Flags) -> bool {
        self.f.contains(flag)
    }

    pub fn set_flag(&mut self, flag: Flags, state: bool) {
        if state {
            self.f |= flag;
        } else {
            self.f &= !flag;
        }
    }

    pub fn new(mode: GBMode) -> Registers {
        match mode {
            GBMode::Classic => {
                Registers {
                    a: 0x01,
                    f: Flags::C | Flags::H | Flags::Z,
                    b: 0x00,
                    c: 0x13,
                    d: 0x00,
                    e: 0xD8,
                    h: 0x01,
                    l: 0x4D,
                    pc: 0x0100,
                    sp: 0xFFFE
                }
            },
            _ => panic!("Mode not yet supported!")
        }
    }
}