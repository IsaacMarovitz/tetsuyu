use crate::mmu::MMU;
use crate::mode::GBMode;
use crate::registers::{Registers, Flags};

pub struct CPU {
    reg: Registers,
    mem: MMU
}

impl CPU {
    pub fn new(mode: GBMode) -> CPU {
        CPU {
            reg: Registers::new(mode),
            mem: MMU::new()
        }
    }

    pub fn call(&mut self, opcode: u8) -> u32 {
        match opcode {
            0x40 => { self.reg.b = self.reg.b;                        1 },
            0x41 => { self.reg.b = self.reg.c;                        1 },
            0x42 => { self.reg.b = self.reg.d;                        1 },
            0x43 => { self.reg.b = self.reg.e;                        1 },
            0x44 => { self.reg.b = self.reg.h;                        1 },
            0x45 => { self.reg.b = self.reg.l;                        1 },
            0x46 => { self.reg.b = self.mem.read(self.reg.get_hl());  2 },
            0x47 => { self.reg.b = self.reg.a;                        1 },
            0x48 => { self.reg.c = self.reg.b;                        1 },
            0x49 => { self.reg.c = self.reg.c;                        1 },
            0x4A => { self.reg.c = self.reg.d;                        1 },
            0x4B => { self.reg.c = self.reg.e;                        1 },
            0x4C => { self.reg.c = self.reg.h;                        1 },
            0x4D => { self.reg.c = self.reg.l;                        1 },
            0x4E => { self.reg.c = self.mem.read(self.reg.get_hl());  2 },
            0x4F => { self.reg.c = self.reg.a;                        1 },
            0x50 => { self.reg.d = self.reg.b;                        1 },
            0x51 => { self.reg.d = self.reg.c;                        1 },
            0x52 => { self.reg.d = self.reg.d;                        1 },
            0x53 => { self.reg.d = self.reg.e;                        1 },
            0x54 => { self.reg.d = self.reg.l;                        1 },
            0x56 => { self.reg.d = self.mem.read(self.reg.get_hl());  2 },
            0x57 => { self.reg.d = self.reg.a;                        1 },
            0x58 => { self.reg.e = self.reg.b;                        1 },
            0x59 => { self.reg.e = self.reg.c;                        1 },
            0x5A => { self.reg.e = self.reg.d;                        1 },
            0x5B => { self.reg.e = self.reg.e;                        1 },
            0x5C => { self.reg.e = self.reg.h;                        1 },
            0x5D => { self.reg.e = self.reg.l;                        1 },
            0x5E => { self.reg.e = self.mem.read(self.reg.get_hl());  2 },
            0x5F => { self.reg.e = self.reg.a;                        1 },
            0x60 => { self.reg.h = self.reg.b;                        1 },
            0x61 => { self.reg.h = self.reg.c;                        1 },
            0x62 => { self.reg.h = self.reg.d;                        1 },
            0x63 => { self.reg.h = self.reg.e;                        1 },
            0x64 => { self.reg.h = self.reg.h;                        1 },
            0x65 => { self.reg.h = self.reg.l;                        1 },
            0x66 => { self.reg.h = self.mem.read(self.reg.get_hl());  2 },
            0x67 => { self.reg.h = self.reg.a;                        1 },
            0x68 => { self.reg.l = self.reg.b;                        1 },
            0x69 => { self.reg.l = self.reg.c;                        1 },
            0x6A => { self.reg.l = self.reg.d;                        1 },
            0x6B => { self.reg.l = self.reg.e;                        1 },
            0x6C => { self.reg.l = self.reg.h;                        1 },
            0x6D => { self.reg.l = self.reg.l;                        1 },
            0x6E => { self.reg.l = self.mem.read(self.reg.get_hl());  2 },
            0x6F => { self.reg.l = self.reg.a;                        1 },
            0x78 => { self.reg.a = self.reg.b;                        1 },
            0x79 => { self.reg.a = self.reg.c;                        1 },
            0x7A => { self.reg.a = self.reg.d;                        1 },
            0x7B => { self.reg.a = self.reg.e;                        1 },
            0x7C => { self.reg.a = self.reg.h;                        1 },
            0x7D => { self.reg.a = self.reg.l;                        1 },
            0x7E => { self.reg.a = self.mem.read(self.reg.get_hl());  2 },
            0x7F => { self.reg.a = self.reg.a;                        1 },
            0x80 => { self.alu_add(self.reg.b);                       1 },
            0x81 => { self.alu_add(self.reg.c);                       1 },
            0x82 => { self.alu_add(self.reg.d);                       1 },
            0x83 => { self.alu_add(self.reg.e);                       1 },
            0x84 => { self.alu_add(self.reg.h);                       1 },
            0x85 => { self.alu_add(self.reg.l);                       1 },
            0x86 => { self.alu_add(self.mem.read(self.reg.get_hl())); 2 },
            0x87 => { self.alu_add(self.reg.a);                       1 },
            0x88 => { self.alu_adc(self.reg.b);                       1 },
            0x89 => { self.alu_adc(self.reg.c);                       1 },
            0x8A => { self.alu_adc(self.reg.d);                       1 },
            0x8B => { self.alu_adc(self.reg.e);                       1 },
            0x8C => { self.alu_adc(self.reg.h);                       1 },
            0x8D => { self.alu_adc(self.reg.l);                       1 },
            0x8E => { self.alu_adc(self.mem.read(self.reg.get_hl())); 2 },
            0x8F => { self.alu_adc(self.reg.a);                       1 },
            0x90 => { self.alu_sub(self.reg.b);                       1 },
            0x91 => { self.alu_sub(self.reg.c);                       1 },
            0x92 => { self.alu_sub(self.reg.d);                       1 },
            0x93 => { self.alu_sub(self.reg.e);                       1 },
            0x94 => { self.alu_sub(self.reg.h);                       1 },
            0x95 => { self.alu_sub(self.reg.l);                       1 },
            0x96 => { self.alu_sub(self.mem.read(self.reg.get_hl())); 2 },
            0x97 => { self.alu_sub(self.reg.a);                       1 },
            0x98 => { self.alu_sbc(self.reg.b);                       1 },
            0x99 => { self.alu_sbc(self.reg.c);                       1 },
            0x9A => { self.alu_sbc(self.reg.d);                       1 },
            0x9B => { self.alu_sbc(self.reg.e);                       1 },
            0x9C => { self.alu_sbc(self.reg.h);                       1 },
            0x9D => { self.alu_sbc(self.reg.l);                       1 },
            0x9E => { self.alu_sbc(self.mem.read(self.reg.get_hl())); 2 },
            0x9F => { self.alu_sbc(self.reg.a);                       1 },
            0xCB => {                                                 0 }, // CB OPs
            code => panic!("Instruction {:2X} is unknown!", code),
        }
    }

    pub fn cb_call(&mut self, opcode: u8) -> u32 {
        match opcode {
            0x40 => { self.alu_bit(self.reg.b, 0); 2 },
            0x41 => { self.alu_bit(self.reg.c, 0); 2 },
            0x42 => { self.alu_bit(self.reg.d, 0); 2 },
            0x43 => { self.alu_bit(self.reg.e, 0); 2 },
            0x44 => { self.alu_bit(self.reg.h, 0); 2 },
            0x45 => { self.alu_bit(self.reg.l, 0); 2 },
            0x46 => {                              4 }, // BIT 0, HL
            0x47 => { self.alu_bit(self.reg.a, 0); 2 },
            0x48 => { self.alu_bit(self.reg.b, 1); 2 },
            0x49 => { self.alu_bit(self.reg.c, 1); 2 },
            0x4A => { self.alu_bit(self.reg.d, 1); 2 },
            0x4B => { self.alu_bit(self.reg.e, 1); 2 },
            0x4C => { self.alu_bit(self.reg.h, 1); 2 },
            0x4D => { self.alu_bit(self.reg.l, 1); 2 },
            0x4E => {                              4 }, // BIT 1, HL
            0x4F => { self.alu_bit(self.reg.a, 1); 2 },
            0x50 => { self.alu_bit(self.reg.b, 2); 2 },
            0x51 => { self.alu_bit(self.reg.c, 2); 2 },
            0x52 => { self.alu_bit(self.reg.d, 2); 2 },
            0x53 => { self.alu_bit(self.reg.e, 2); 2 },
            0x54 => { self.alu_bit(self.reg.h, 2); 2 },
            0x55 => { self.alu_bit(self.reg.l, 2); 2 },
            0x56 => {                              4 }, // BIT 2, HL
            0x57 => { self.alu_bit(self.reg.a, 2); 2 },
            0x58 => { self.alu_bit(self.reg.b, 3); 2 },
            0x59 => { self.alu_bit(self.reg.c, 3); 2 },
            0x5A => { self.alu_bit(self.reg.d, 3); 2 },
            0x5B => { self.alu_bit(self.reg.e, 3); 2 },
            0x5C => { self.alu_bit(self.reg.h, 3); 2 },
            0x5D => { self.alu_bit(self.reg.l, 3); 2 },
            0x5E => {                              4 }, // BIT 3, HL
            0x5F => { self.alu_bit(self.reg.a, 3); 2 },
            0x60 => { self.alu_bit(self.reg.b, 4); 2 },
            0x61 => { self.alu_bit(self.reg.c, 4); 2 },
            0x62 => { self.alu_bit(self.reg.d, 4); 2 },
            0x63 => { self.alu_bit(self.reg.e, 4); 2 },
            0x64 => { self.alu_bit(self.reg.h, 4); 2 },
            0x65 => { self.alu_bit(self.reg.l, 4); 2 },
            0x66 => {                              4 }, // BIT 4, HL
            0x67 => { self.alu_bit(self.reg.a, 4); 2 },
            0x68 => { self.alu_bit(self.reg.b, 5); 2 },
            0x69 => { self.alu_bit(self.reg.c, 5); 2 },
            0x6A => { self.alu_bit(self.reg.d, 5); 2 },
            0x6B => { self.alu_bit(self.reg.e, 5); 2 },
            0x6C => { self.alu_bit(self.reg.h, 5); 2 },
            0x6D => { self.alu_bit(self.reg.l, 5); 2 },
            0x6E => {                              4 }, // BIT 5, HL
            0x6F => { self.alu_bit(self.reg.a, 5); 2 },
            0x70 => { self.alu_bit(self.reg.b, 6); 2 },
            0x71 => { self.alu_bit(self.reg.c, 6); 2 },
            0x72 => { self.alu_bit(self.reg.d, 6); 2 },
            0x73 => { self.alu_bit(self.reg.e, 6); 2 },
            0x74 => { self.alu_bit(self.reg.h, 6); 2 },
            0x75 => { self.alu_bit(self.reg.l, 6); 2 },
            0x76 => {                              4 }, // BIT 6, HL
            0x77 => { self.alu_bit(self.reg.a, 6); 2 },
            0x78 => { self.alu_bit(self.reg.b, 7); 2 },
            0x79 => { self.alu_bit(self.reg.c, 7); 2 },
            0x7A => { self.alu_bit(self.reg.d, 7); 2 },
            0x7B => { self.alu_bit(self.reg.e, 7); 2 },
            0x7C => { self.alu_bit(self.reg.h, 7); 2 },
            0x7D => { self.alu_bit(self.reg.l, 7); 2 },
            0x7E => {                              4 }, // BIT 7, HL
            0x7F => { self.alu_bit(self.reg.a, 7); 2 },
            code => panic!("CB Instruction {:2X} is unknown!", code),
        }
    }

    fn alu_add(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a.wrapping_add(x);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) > u16::from(u8::MAX));
        self.reg.set_flag(Flags::H, (a & 0x0F) + (a & 0x0F) > 0x0F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0x00);
        self.reg.a = r;
    }

    fn alu_adc(&mut self, x: u8) {
        let a = self.reg.a;
        let c = u8::from(self.reg.get_flag(Flags::C));
        let r = a.wrapping_add(x).wrapping_add(c);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) + u16::from(c) > u16::from(u8::MAX));
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

    fn alu_bit(&mut self, a: u8, b: u8) {
        let r = a & (1 << b) == 0x00;
        self.reg.set_flag(Flags::H, true);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r);
    }
}