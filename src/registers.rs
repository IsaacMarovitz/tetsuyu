use bitflags::bitflags;
use crate::mode::GBMode;

pub struct Registers {
    pub a: u8,
    f: u8,
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
    // Registers can be paired to be used for 16-bit operations
    // A+F, B+C, D+E, H+L
    pub fn get_af(&self) -> u16 {
        (u16::from(self.a) << 8) | u16::from(self.f)
    }

    pub fn get_bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    pub fn get_de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    pub fn get_hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
    }

    pub fn set_af(&mut self, x: u16) {
        self.a = (x >> 8) as u8;
        self.f = (x & 0x00F0) as u8;
    }

    pub fn set_bc(&mut self, x: u16) {
        self.b = (x >> 8) as u8;
        self.c = (x & 0x00FF) as u8;
    }

    pub fn set_de(&mut self, x: u16) {
        self.d = (x >> 8) as u8;
        self.e = (x & 0x00FF) as u8;
    }

    pub fn set_hl(&mut self, x: u16) {
        self.h = (x >> 8) as u8;
        self.l = (x & 0x00FF) as u8;
    }

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
                    f: (Flags::C | Flags::H | Flags::Z).bits(),
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