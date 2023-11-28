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
            0x06 => {},
            0x0E => {},
            0x16 => {},
            0x1E => {},
            0x26 => {},
            0x2E => {},
            0x36 => {},
            0x3E => {},
            0x02 => {},
            0x12 => {},
            0x0A => {},
            0x1A => {},
            0x22 => {},
            0x32 => {},
            0x2A => {},
            0x3A => {},
            0x40 => self.reg.b = self.reg.b,
            0x41 => self.reg.b = self.reg.c,
            0x42 => self.reg.b = self.reg.d,
            0x43 => self.reg.b = self.reg.e,
            0x44 => self.reg.b = self.reg.h,
            0x45 => self.reg.b = self.reg.l,
            0x46 => {}, // LD B, HL
            0x47 => self.reg.b = self.reg.a,
            0x48 => self.reg.c = self.reg.b,
            0x49 => self.reg.c = self.reg.c,
            0x4A => self.reg.c = self.reg.d,
            0x4B => self.reg.c = self.reg.e,
            0x4C => self.reg.c = self.reg.h,
            0x4D => self.reg.c = self.reg.l,
            0x4E => {}, // LD C, HL
            0x4F => self.reg.c = self.reg.a,
            0x50 => self.reg.d = self.reg.b,
            0x51 => self.reg.d = self.reg.c,
            0x52 => self.reg.d = self.reg.d,
            0x53 => self.reg.d = self.reg.e,
            0x54 => self.reg.d = self.reg.l,
            0x56 => {}, // LD D, HL
            0x57 => self.reg.d = self.reg.a,
            0x58 => self.reg.e = self.reg.b,
            0x59 => self.reg.e = self.reg.c,
            0x5A => self.reg.e = self.reg.d,
            0x5B => self.reg.e = self.reg.e,
            0x5C => self.reg.e = self.reg.h,
            0x5D => self.reg.e = self.reg.l,
            0x5E => {}, // LD E, HL
            0x5F => self.reg.e = self.reg.a,
            0x60 => self.reg.h = self.reg.b,
            0x61 => self.reg.h = self.reg.c,
            0x62 => self.reg.h = self.reg.d,
            0x63 => self.reg.h = self.reg.e,
            0x64 => self.reg.h = self.reg.h,
            0x65 => self.reg.h = self.reg.l,
            0x66 => {}, // LD H, HL
            0x67 => self.reg.h = self.reg.a,
            0x68 => self.reg.l = self.reg.b,
            0x69 => self.reg.l = self.reg.c,
            0x6A => self.reg.l = self.reg.d,
            0x6B => self.reg.l = self.reg.e,
            0x6C => self.reg.l = self.reg.h,
            0x6D => self.reg.l = self.reg.l,
            0x6E => {}, // LD L, HL,
            0x6F => self.reg.l = self.reg.a,
            0x78 => self.reg.a = self.reg.b,
            0x79 => self.reg.a = self.reg.c,
            0x7A => self.reg.a = self.reg.d,
            0x7B => self.reg.a = self.reg.e,
            0x7C => self.reg.a = self.reg.h,
            0x7D => self.reg.a = self.reg.l,
            0x7E => {}, // LD A, HL
            0x7F => self.reg.a = self.reg.a,
            0xCB => {}, // CB OPs
            code => panic!("Instruction {:2X} is unknown!", code),
        }
    }

    fn alu_add(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a.wrapping_add(x);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) > u16::from(u8::max));
        self.reg.set_flag(Flags::H, (a & 0x0F) + (a & 0x0F) > 0x0F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0x00);
        self.reg.a = r;
    }

    fn alu_adc(&mut self, x: u8) {
        let a = self.reg.a;
        let c = u8::from(self.reg.get_flag(Flags::C));
        let r = a.wrapping_add(x).wrapping_add(c);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) + u16::from(c) > u16::from(u8::max));
        self.reg.set_flag(Flags::H, (a & 0x0F) + (a & 0x0F)  + (c & 0x0F) > 0x0F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0x00);
        self.reg.a = r;
    }

    fn alu_sub(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a.wrapping_sub(x);
        self.reg.set_flag(Flags::C, u16::from(a) < u16::from(x));
        self.reg.set_flag(Flags::H, (a & 0xF) < (x & 0xF));
        self.reg.set_flag(Flags::N, true);
        self.reg.set_flag(Flags::Z, r == 0x00);
        self.reg.a = r;
    }

    fn alu_sbc(&mut self, x: u8) {
        let a = self.reg.a;
        let c = u8::from(self.reg.get_flag(Flags::C));
        let r = a.wrapping_sub(x).wrapping_sub(c);
        self.reg.set_flag(Flags::C, u16::from(a) < u16::from(x) + u16::from(c));
        self.reg.set_flag(Flags::H, (a & 0xF) < (x & 0xF) + c);
        self.reg.set_flag(Flags::N, true);
        self.reg.set_flag(Flags::Z, r == 0x00);
        self.reg.a = r;
    }
}