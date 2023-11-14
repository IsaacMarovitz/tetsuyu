use crate::mode::GBMode;
use crate::registers::{Registers, Flags};

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
            code => panic!("Instruction {:2X} is unknown!", code),
        }
    }

    fn alu_add(&mut self, x: u8) {
        let a = self.reg.a;
        let x = a.wrapping_add(x);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) > 0xFF);
        self.reg.set_flag(Flags::H, (a & 0x0F) + (a & 0x0F) > 0x0F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, x == 0x00);
        self.reg.a = x;
    }
}