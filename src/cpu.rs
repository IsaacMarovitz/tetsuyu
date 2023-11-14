use bitflags::{bitflags};

pub struct Registers {
    a: u8,
    f: Flags,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
}

bitflags! {
    pub struct Flags: u8 {
        const C = 0b00010000;
        const H = 0b00100000;
        const N = 0b01000000;
        const Z = 0b10000000;
    }
}

pub enum GBMode {
    Classic,
    Color,
    ColorAsClassic,
}

impl Registers {
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

pub struct CPU {
    reg: Registers
}

impl CPU {
    pub fn new(mode: GBMode) -> CPU {
        CPU {
            reg: Registers::new(mode)
        }
    }
}