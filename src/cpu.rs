use crate::mmu::MMU;
use crate::mode::GBMode;
use crate::registers::{Registers, Flags};

pub struct CPU {
    reg: Registers,
    mem: MMU
}

impl CPU {
    pub fn new(mode: GBMode, rom: [u8; 0x8000]) -> CPU {
        CPU {
            reg: Registers::new(mode),
            mem: MMU::new(rom)
        }
    }

    pub fn cycle(&mut self) -> u32 {
        self.call()
    }

    pub fn read_byte(&mut self) -> u8 {
        let byte = self.mem.read(self.reg.pc);
        self.reg.pc += 1;
        byte
    }

    pub fn call(&mut self) -> u32 {
        let opcode = self.read_byte();
        match opcode {
            0x00 => { 1 },
            0x02 => { self.mem.write(self.reg.get_bc(), self.reg.a);  2 },
            0x03 => { let bc = self.reg.get_bc();
                      self.reg.set_bc(bc.wrapping_add(1));            2 },
            0x06 => { self.reg.b = self.read_byte();                  2 },
            0x0A => { self.reg.a = self.mem.read(self.reg.get_bc());  2 },
            0x0B => { let bc = self.reg.get_bc();
                      self.reg.set_bc(bc.wrapping_sub(1));            2 },
            0x0E => { self.reg.c = self.read_byte();                  2 },
            0x12 => { self.mem.write(self.reg.get_de(), self.reg.a);  2 },
            0x13 => { let de = self.reg.get_de();
                      self.reg.set_de(de.wrapping_add(1));            2 },
            0x16 => { self.reg.d = self.read_byte();                  2 },
            0x18 => { self.reg.pc += self.read_byte() as u16;         3 },
            0x1A => { self.reg.a = self.mem.read(self.reg.get_de());  2 },
            0x1B => { let de = self.reg.get_de();
                      self.reg.set_de(de.wrapping_sub(1));            2 },
            0x1E => { self.reg.e = self.read_byte();                  2 },
            0x20 => { if !self.reg.get_flag(Flags::Z)
                      { self.reg.pc += self.read_byte() as u16;       3 }
                      else { self.reg.pc += 1;                        2 }
                    },
            0x22 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.a);
                      self.reg.set_hl(a + 1);                         2 },
            0x23 => { let hl = self.reg.get_hl();
                      self.reg.set_hl(hl.wrapping_add(1));            2 },
            0x26 => { self.reg.h = self.read_byte();                  2 },
            0x28 => { if self.reg.get_flag(Flags::Z)
                      { self.reg.pc += self.read_byte() as u16;       3 }
                      else { self.reg.pc += 1;                        2 }
                    },
            0x2A => { let a = self.reg.get_hl();
                      self.reg.a = self.mem.read(a);
                      self.reg.set_hl(a + 1);                         2 },
            0x2B => { let hl = self.reg.get_hl();
                      self.reg.set_hl(hl.wrapping_sub(1));            2 },
            0x2E => { self.reg.l = self.read_byte();                  2 },
            0x30 => { if !self.reg.get_flag(Flags::C)
                      { self.reg.pc += self.read_byte() as u16;       3 }
                      else { self.reg.pc += 1;                        2 }
                    },
            0x32 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.a);
                      self.reg.set_hl(a - 1);                         2 },
            0x33 => { let sp = self.reg.sp;
                      self.reg.sp = sp.wrapping_add(1);               2 },
            0x36 => { let a = self.reg.get_hl();
                      let b = self.read_byte();
                      self.mem.write(a, b);                           3 },
            0x38 => { if self.reg.get_flag(Flags::C)
                      { self.reg.pc += self.read_byte() as u16;       3 }
                      else { self.reg.pc += 1;                        2 }
                    },
            0x3A => { let a = self.reg.get_hl();
                      self.reg.a = self.mem.read(a);
                      self.reg.set_hl(a - 1);                         2 },
            0x3B => { let sp = self.reg.sp;
                      self.reg.sp = sp.wrapping_sub(1);               2 },
            0x3E => { self.reg.a = self.read_byte();                  2 },
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
            0x70 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.b);                  2 },
            0x71 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.c);                  2 },
            0x72 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.d);                  2 },
            0x73 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.e);                  2 },
            0x74 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.h);                  2 },
            0x75 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.l);                  2 },
            0x77 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.a);                  2 },
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
            0xA0 => { self.alu_add(self.reg.b);                       1 },
            0xA1 => { self.alu_add(self.reg.c);                       1 },
            0xA2 => { self.alu_add(self.reg.d);                       1 },
            0xA3 => { self.alu_add(self.reg.e);                       1 },
            0xA4 => { self.alu_add(self.reg.h);                       1 },
            0xA5 => { self.alu_add(self.reg.l);                       1 },
            0xA6 => { self.alu_add(self.mem.read(self.reg.get_hl())); 2 },
            0xA7 => { self.alu_add(self.reg.a);                       1 },
            0xA8 => { self.alu_xor(self.reg.b);                       1 },
            0xA9 => { self.alu_xor(self.reg.c);                       1 },
            0xAA => { self.alu_xor(self.reg.d);                       1 },
            0xAB => { self.alu_xor(self.reg.e);                       1 },
            0xAC => { self.alu_xor(self.reg.h);                       1 },
            0xAD => { self.alu_xor(self.reg.l);                       1 },
            0xAE => { self.alu_xor(self.mem.read(self.reg.get_hl())); 2 },
            0xAF => { self.alu_xor(self.reg.a);                       1 },
            0xB0 => { self.alu_or(self.reg.b);                        1 },
            0xB1 => { self.alu_or(self.reg.c);                        1 },
            0xB2 => { self.alu_or(self.reg.d);                        1 },
            0xB3 => { self.alu_or(self.reg.e);                        1 },
            0xB4 => { self.alu_or(self.reg.h);                        1 },
            0xB5 => { self.alu_or(self.reg.l);                        1 },
            0xB6 => { self.alu_or(self.mem.read(self.reg.get_hl()));  2 },
            0xB7 => { self.alu_or(self.reg.a);                        1 },
            0xB8 => { self.alu_cp(self.reg.b);                        1 },
            0xB9 => { self.alu_cp(self.reg.c);                        1 },
            0xBA => { self.alu_cp(self.reg.d);                        1 },
            0xBB => { self.alu_cp(self.reg.e);                        1 },
            0xBC => { self.alu_cp(self.reg.h);                        1 },
            0xBD => { self.alu_cp(self.reg.l);                        1 },
            0xBE => { self.alu_cp(self.mem.read(self.reg.get_hl()));  2 },
            0xBF => { self.alu_cp(self.reg.a);                        1 },
            0xCB => { self.cb_call()                                    },
            0xD6 => { let b = self.read_byte();
                      self.alu_sub(b);                                2 },
            0xE6 => { let b = self.read_byte();
                      self.alu_and(b);                                2 },
            0xEE => { let b = self.read_byte();
                      self.alu_xor(b);                                2 },
            0xE9 => { self.reg.pc = self.reg.get_hl();                1 },
            0xF6 => { let b = self.read_byte();
                      self.alu_or(b);                                 2 },
            0xFE => { let b = self.read_byte();
                      self.alu_cp(b);                                 2 },
            // Should be a panic!, keep it as a println! for now
            code => { println!("Instruction {:#04x} is unknown!", code);  0 },
        }
    }

    pub fn cb_call(&mut self) -> u32 {
        let opcode = self.read_byte();
        match opcode {
            0x40 => { self.alu_bit(self.reg.b, 0);       2 },
            0x41 => { self.alu_bit(self.reg.c, 0);       2 },
            0x42 => { self.alu_bit(self.reg.d, 0);       2 },
            0x43 => { self.alu_bit(self.reg.e, 0);       2 },
            0x44 => { self.alu_bit(self.reg.h, 0);       2 },
            0x45 => { self.alu_bit(self.reg.l, 0);       2 },
            0x46 => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 0); 4 },
            0x47 => { self.alu_bit(self.reg.a, 0);       2 },
            0x48 => { self.alu_bit(self.reg.b, 1);       2 },
            0x49 => { self.alu_bit(self.reg.c, 1);       2 },
            0x4A => { self.alu_bit(self.reg.d, 1);       2 },
            0x4B => { self.alu_bit(self.reg.e, 1);       2 },
            0x4C => { self.alu_bit(self.reg.h, 1);       2 },
            0x4D => { self.alu_bit(self.reg.l, 1);       2 },
            0x4E => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 1); 4 },
            0x4F => { self.alu_bit(self.reg.a, 1);       2 },
            0x50 => { self.alu_bit(self.reg.b, 2);       2 },
            0x51 => { self.alu_bit(self.reg.c, 2);       2 },
            0x52 => { self.alu_bit(self.reg.d, 2);       2 },
            0x53 => { self.alu_bit(self.reg.e, 2);       2 },
            0x54 => { self.alu_bit(self.reg.h, 2);       2 },
            0x55 => { self.alu_bit(self.reg.l, 2);       2 },
            0x56 => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 2); 4 },
            0x57 => { self.alu_bit(self.reg.a, 2);       2 },
            0x58 => { self.alu_bit(self.reg.b, 3);       2 },
            0x59 => { self.alu_bit(self.reg.c, 3);       2 },
            0x5A => { self.alu_bit(self.reg.d, 3);       2 },
            0x5B => { self.alu_bit(self.reg.e, 3);       2 },
            0x5C => { self.alu_bit(self.reg.h, 3);       2 },
            0x5D => { self.alu_bit(self.reg.l, 3);       2 },
            0x5E => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 3); 4 },
            0x5F => { self.alu_bit(self.reg.a, 3);       2 },
            0x60 => { self.alu_bit(self.reg.b, 4);       2 },
            0x61 => { self.alu_bit(self.reg.c, 4);       2 },
            0x62 => { self.alu_bit(self.reg.d, 4);       2 },
            0x63 => { self.alu_bit(self.reg.e, 4);       2 },
            0x64 => { self.alu_bit(self.reg.h, 4);       2 },
            0x65 => { self.alu_bit(self.reg.l, 4);       2 },
            0x66 => { let a = self.reg.get_hl();
                self.alu_bit(self.mem.read(a), 4);       4 },
            0x67 => { self.alu_bit(self.reg.a, 4);       2 },
            0x68 => { self.alu_bit(self.reg.b, 5);       2 },
            0x69 => { self.alu_bit(self.reg.c, 5);       2 },
            0x6A => { self.alu_bit(self.reg.d, 5);       2 },
            0x6B => { self.alu_bit(self.reg.e, 5);       2 },
            0x6C => { self.alu_bit(self.reg.h, 5);       2 },
            0x6D => { self.alu_bit(self.reg.l, 5);       2 },
            0x6E => { let a = self.reg.get_hl();
                self.alu_bit(self.mem.read(a), 5);       4 },
            0x6F => { self.alu_bit(self.reg.a, 5);       2 },
            0x70 => { self.alu_bit(self.reg.b, 6);       2 },
            0x71 => { self.alu_bit(self.reg.c, 6);       2 },
            0x72 => { self.alu_bit(self.reg.d, 6);       2 },
            0x73 => { self.alu_bit(self.reg.e, 6);       2 },
            0x74 => { self.alu_bit(self.reg.h, 6);       2 },
            0x75 => { self.alu_bit(self.reg.l, 6);       2 },
            0x76 => { let a = self.reg.get_hl();
                self.alu_bit(self.mem.read(a), 6);       4 },
            0x77 => { self.alu_bit(self.reg.a, 6);       2 },
            0x78 => { self.alu_bit(self.reg.b, 7);       2 },
            0x79 => { self.alu_bit(self.reg.c, 7);       2 },
            0x7A => { self.alu_bit(self.reg.d, 7);       2 },
            0x7B => { self.alu_bit(self.reg.e, 7);       2 },
            0x7C => { self.alu_bit(self.reg.h, 7);       2 },
            0x7D => { self.alu_bit(self.reg.l, 7);       2 },
            0x7E => { let a = self.reg.get_hl();
                self.alu_bit(self.mem.read(a), 7);       4 },
            0x7F => { self.alu_bit(self.reg.a, 7);       2 },
            // Should be a panic!, keep it as a println! for now
            code => { println!("CB Instruction {:#04x} is unknown!", code); 0 },
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

    fn alu_and(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a & x;
        self.reg.set_flag(Flags::C, false);
        self.reg.set_flag(Flags::H, true);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.a = r;
    }

    fn alu_or(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a | x;
        self.reg.set_flag(Flags::C, false);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.a = r;
    }

    fn alu_xor(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a ^ x;
        self.reg.set_flag(Flags::C, false);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.a = r;
    }

    fn alu_cp(&mut self, x: u8) {
        let r = self.reg.a;
        self.alu_sub(x);
        self.reg.a = r
    }

    fn alu_bit(&mut self, a: u8, b: u8) {
        let r = a & (1 << b) == 0x00;
        self.reg.set_flag(Flags::H, true);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r);
    }
}