use crate::mode::GBMode;
use crate::registers::Registers;

pub struct CPU {
    reg: Registers
}

impl CPU {
    pub fn new(mode: GBMode) -> CPU {
        CPU {
            reg: Registers::new(mode)
        }
    }

    pub fn call(&mut self, opcode: u8) {
        match opcode {
            0x06 => self.reg.b = 0,
            0x0e => self.reg.c = 0,
            0x16 => self.reg.d = 0,
            0x1e => self.reg.e = 0,
            0x26 => self.reg.h = 0,
            0x2e => self.reg.l = 0,
            0x36 => {},
            0x3e => {},
            0x02 => {},
            0x12 => {},
            0x0a => {},
            0x1a => {},
            0x22 => {},
            0x32 => {},
            0x2a => {},
            0x3a => {},
            0x40 => {},
            0x41 => self.reg.b = self.reg.c,
            0x42 => self.reg.b = self.reg.d,
            0x43 => self.reg.b = self.reg.e,
            0x44 => self.reg.b = self.reg.h,
            0x45 => self.reg.b = self.reg.l,
            0x46 => {},
            0x47 => self.reg.b = self.reg.a,
            _ => {},
        }
    }
}