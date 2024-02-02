use std::fs::File;
use std::io::Read;
use std::process;
use crate::config::Config;
use crate::components::prelude::*;

pub struct CPU {
    pub reg: Registers,
    pub mem: MMU,
    halted: bool,
    // Enabled Interrupts
    ime: bool,
    ime_ask: bool
}

impl CPU {
    pub fn new(mut rom: Vec<u8>, config: Config) -> Self {
        let booting: bool = match config.boot_rom {
            Some(ref path) => {
                let mut boot_rom = Vec::new();
                let mut boot = match File::open(path.clone()) {
                    Ok(file) => file,
                    Err(err) => {
                        eprintln!("Failed to open Boot ROM at \"{}\": {}", path.clone(), err);
                        process::exit(1);
                    }
                };
                boot.read_to_end(&mut boot_rom)
                    .expect("Failed to read Boot ROM!");

                // Copy Boot ROM
                if config.mode == GBMode::DMG {
                    rom[0..=0x00FF].copy_from_slice(boot_rom.as_slice());
                } else if config.mode == GBMode::CGB {
                    rom[0..=0x08FF].copy_from_slice(boot_rom.as_slice());
                }

                true
            }
            None => false,
        };

        Self {
            reg: Registers::new(config.clone().mode, booting),
            mem: MMU::new(rom, config),
            halted: false,
            ime: false,
            ime_ask: false
        }
    }

    pub fn cycle(&mut self) -> u32 {
        let cycles = {
            let count = self.interrupt();
            if count != 0 {
                count
            } else if self.halted {
                1
            } else {
                if self.ime_ask && !self.ime {
                    self.ime = true;
                    self.ime_ask = false;
                }

                self.op_call()
            }
        };
        cycles * 4
    }

    fn interrupt(&mut self) -> u32 {
        let intf = self.mem.read(0xFF0F);
        let inte = self.mem.read(0xFFFF);
        let triggered = intf & inte;
        if triggered == 0 {
            return 0;
        }

        self.halted = false;
        if !self.ime {
            return 0;
        }
        self.ime = false;

        let n = triggered.trailing_zeros();
        let remaining = intf & !(1 << n);
        self.mem.write(0xFF0F, remaining);

        self.push(self.reg.pc);
        self.reg.pc = 0x0040 | ((n as u16) << 3);
        4
    }

    pub fn read_byte(&mut self) -> u8 {
        let byte = self.mem.read(self.reg.pc);
        self.reg.pc += 1;
        byte
    }

    pub fn read_word(&mut self) -> u16 {
        let word = self.mem.read_word(self.reg.pc);
        self.reg.pc += 2;
        word
    }

    pub fn push(&mut self, v: u16) {
        self.reg.sp = self.reg.sp.wrapping_sub(2);
        self.mem.write_word(self.reg.sp, v);
    }

    pub fn pop(&mut self) -> u16 {
        let word = self.mem.read_word(self.reg.sp);
        self.reg.sp += 2;
        word
    }

    pub fn op_call(&mut self) -> u32 {
        let opcode = self.read_byte();
        match opcode {
            0x00 => { 1 },
            0x01 => { let v = self.read_word();
                      self.reg.set_bc(v);                             3 },
            0x02 => { self.mem.write(self.reg.get_bc(), self.reg.a);  2 },
            0x03 => { let bc = self.reg.get_bc();
                      self.reg.set_bc(bc.wrapping_add(1));            2 },
            0x04 => { self.reg.b = self.alu_inc(self.reg.b);          1 },
            0x05 => { self.reg.b = self.alu_dec(self.reg.b);          1 },
            0x06 => { self.reg.b = self.read_byte();                  2 },
            0x07 => { self.reg.a = self.alu_rlc(self.reg.a);
                      self.reg.set_flag(Flags::Z, false);             1 },
            0x08 => { let a = self.read_word();
                      self.mem.write_word(a, self.reg.sp);            5 },
            0x09 => { self.alu_add_16(self.reg.get_bc());             2 },
            0x0A => { self.reg.a = self.mem.read(self.reg.get_bc());  2 },
            0x0B => { let bc = self.reg.get_bc();
                      self.reg.set_bc(bc.wrapping_sub(1));            2 },
            0x0C => { self.reg.c = self.alu_inc(self.reg.c);          1 },
            0x0D => { self.reg.c = self.alu_dec(self.reg.c);          1 },
            0x0E => { self.reg.c = self.read_byte();                  2 },
            0x0F => { self.reg.a = self.alu_rrc(self.reg.a);
                      self.reg.set_flag(Flags::Z, false);             1 },
            0x10 => {                                                 1 },
            0x11 => { let v = self.read_word();
                      self.reg.set_de(v);                             3 },
            0x12 => { self.mem.write(self.reg.get_de(), self.reg.a);  2 },
            0x13 => { let de = self.reg.get_de();
                      self.reg.set_de(de.wrapping_add(1));            2 },
            0x14 => { self.reg.d = self.alu_inc(self.reg.d);          1 },
            0x15 => { self.reg.d = self.alu_dec(self.reg.d);          1 },
            0x16 => { self.reg.d = self.read_byte();                  2 },
            0x17 => { self.reg.a = self.alu_rl(self.reg.a);
                      self.reg.set_flag(Flags::Z, false);             1 },
            0x18 => { self.jr(true);                                  3 },
            0x19 => { self.alu_add_16(self.reg.get_de());             2 },
            0x1A => { self.reg.a = self.mem.read(self.reg.get_de());  2 },
            0x1B => { let de = self.reg.get_de();
                      self.reg.set_de(de.wrapping_sub(1));            2 },
            0x1C => { self.reg.e = self.alu_inc(self.reg.e);          1 },
            0x1D => { self.reg.e = self.alu_dec(self.reg.e);          1 },
            0x1E => { self.reg.e = self.read_byte();                  2 },
            0x1F => { self.reg.a = self.alu_rr(self.reg.a);
                      self.reg.set_flag(Flags::Z, false);             1 },
            0x20 => { self.jr(!self.reg.get_flag(Flags::Z))             },
            0x21 => { let v = self.read_word();
                      self.reg.set_hl(v);                             3 },
            0x22 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.a);
                      self.reg.set_hl(a + 1);                         2 },
            0x23 => { let hl = self.reg.get_hl();
                      self.reg.set_hl(hl.wrapping_add(1));            2 },
            0x24 => { self.reg.h = self.alu_inc(self.reg.h);          1 },
            0x25 => { self.reg.h = self.alu_dec(self.reg.h);          1 },
            0x26 => { self.reg.h = self.read_byte();                  2 },
            0x27 => { self.alu_daa();                                 1 },
            0x28 => { self.jr(self.reg.get_flag(Flags::Z))              },
            0x29 => { self.alu_add_16(self.reg.get_hl());             2 },
            0x2A => { let a = self.reg.get_hl();
                      self.reg.a = self.mem.read(a);
                      self.reg.set_hl(a + 1);                         2 },
            0x2B => { let hl = self.reg.get_hl();
                      self.reg.set_hl(hl.wrapping_sub(1));            2 },
            0x2C => { self.reg.l = self.alu_inc(self.reg.l);          1 },
            0x2D => { self.reg.l = self.alu_dec(self.reg.l);          1 },
            0x2E => { self.reg.l = self.read_byte();                  2 },
            0x2F => { self.alu_cpl();                                 1 },
            0x30 => { self.jr(!self.reg.get_flag(Flags::C))             },
            0x31 => { let v = self.read_word();
                      self.reg.sp = v;                                3 },
            0x32 => { let a = self.reg.get_hl();
                      self.mem.write(a, self.reg.a);
                      self.reg.set_hl(a - 1);                         2 },
            0x33 => { let sp = self.reg.sp;
                      self.reg.sp = sp.wrapping_add(1);               2 },
            0x34 => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_inc(v);
                      self.mem.write(a, v);                           3 },
            0x35 => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_dec(v);
                      self.mem.write(a, v);                           3 },
            0x36 => { let a = self.reg.get_hl();
                      let b = self.read_byte();
                      self.mem.write(a, b);                           3 },
            0x37 => { self.alu_scf();                                 1 },
            0x38 => { self.jr(self.reg.get_flag(Flags::C))              },
            0x39 => { self.alu_add_16(self.reg.sp);                   2 },
            0x3A => { let a = self.reg.get_hl();
                      self.reg.a = self.mem.read(a);
                      self.reg.set_hl(a - 1);                         2 },
            0x3B => { let sp = self.reg.sp;
                      self.reg.sp = sp.wrapping_sub(1);               2 },
            0x3C => { self.reg.a = self.alu_inc(self.reg.a);          1 },
            0x3D => { self.reg.a = self.alu_dec(self.reg.a);          1 },
            0x3E => { self.reg.a = self.read_byte();                  2 },
            0x3F => { self.alu_ccf();                                 1 },
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
            0x54 => { self.reg.d = self.reg.h;                        1 },
            0x55 => { self.reg.d = self.reg.l;                        1 },
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
            0x76 => { self.halted = true;                             1 },
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
            0xA0 => { self.alu_and(self.reg.b);                       1 },
            0xA1 => { self.alu_and(self.reg.c);                       1 },
            0xA2 => { self.alu_and(self.reg.d);                       1 },
            0xA3 => { self.alu_and(self.reg.e);                       1 },
            0xA4 => { self.alu_and(self.reg.h);                       1 },
            0xA5 => { self.alu_and(self.reg.l);                       1 },
            0xA6 => { self.alu_and(self.mem.read(self.reg.get_hl())); 2 },
            0xA7 => { self.alu_and(self.reg.a);                       1 },
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
            0xC0 => { self.ret(!self.reg.get_flag(Flags::Z))            },
            0xC1 => { let v = self.pop();
                      self.reg.set_bc(v);                             3 },
            0xC2 => { self.jp(!self.reg.get_flag(Flags::Z))             },
            0xC3 => { self.reg.pc = self.read_word();                 4 },
            0xC4 => { self.call(!self.reg.get_flag(Flags::Z))           },
            0xC5 => { self.push(self.reg.get_bc());                   4 },
            0xC6 => { let v = self.read_byte();
                      self.alu_add(v);                                2 },
            0xC7 => { self.rst(0x00)                                    },
            0xC8 => { self.ret(self.reg.get_flag(Flags::Z))             },
            0xC9 => { self.reg.pc = self.pop();                       4 },
            0xCA => { self.jp(self.reg.get_flag(Flags::Z))              },
            0xCB => { self.cb_call()                                    },
            0xCC => { self.call(self.reg.get_flag(Flags::Z))            },
            0xCD => { self.push(self.reg.pc + 2);
                      self.reg.pc = self.read_word();                 6 },
            0xCE => { let v = self.read_byte();
                      self.alu_adc(v);                                2 },
            0xCF => { self.rst(0x08)                                    },
            0xD0 => { self.ret(!self.reg.get_flag(Flags::C))            },
            0xD1 => { let v = self.pop();
                      self.reg.set_de(v);                             3 },
            0xD2 => { self.jp(!self.reg.get_flag(Flags::C))             },
            0xD4 => { self.call(!self.reg.get_flag(Flags::C))           },
            0xD5 => { self.push(self.reg.get_de());                   4 },
            0xD6 => { let b = self.read_byte();
                      self.alu_sub(b);                                2 },
            0xD7 => { self.rst(0x10)                                    },
            0xD8 => { self.ret(self.reg.get_flag(Flags::C))             },
            0xD9 => { self.reg.pc = self.pop();
                      self.ime = true;                                4 },
            0xDA => { self.jp(self.reg.get_flag(Flags::C))              },
            0xDC => { self.call(self.reg.get_flag(Flags::C))            },
            0xDE => { let v = self.read_byte();
                      self.alu_sbc(v);                                2 },
            0xDF => { self.rst(0x18)                                    },
            0xE0 => { let a = 0xFF00 | u16::from(self.read_byte());
                      self.mem.write(a, self.reg.a);                  3 },
            0xE1 => { let v = self.pop();
                      self.reg.set_hl(v);                             3 },
            0xE2 => { let a = 0xFF00 | u16::from(self.reg.c);
                      self.mem.write(a, self.reg.a);                  2 },
            0xE5 => { self.push(self.reg.get_hl());                   4 },
            0xE6 => { let b = self.read_byte();
                      self.alu_and(b);                                2 },
            0xE7 => { self.rst(0x20)                                    },
            0xE8 => { self.reg.sp = self.alu_add_16_imm(self.reg.sp); 4 },
            0xE9 => { self.reg.pc = self.reg.get_hl();                1 },
            0xEA => { let a = self.read_word();
                      self.mem.write(a, self.reg.a);                  4 },
            0xEE => { let b = self.read_byte();
                      self.alu_xor(b);                                2 },
            0xEF => { self.rst(0x28)                                    },
            0xF0 => { let a = 0xFF00 | u16::from(self.read_byte());
                      self.reg.a = self.mem.read(a);                  3 },
            0xF1 => { let v = self.pop();
                      self.reg.set_af(v);                             3 },
            0xF2 => { let a = 0xFF00 | u16::from(self.reg.c);
                      self.reg.a = self.mem.read(a);                  2 },
            0xF3 => { self.ime = false; self.ime_ask = false;         1 },
            0xF5 => { self.push(self.reg.get_af());                   4 },
            0xF6 => { let b = self.read_byte();
                      self.alu_or(b);                                 2 },
            0xF7 => { self.rst(0x30)                                    },
            0xF8 => { let v = self.alu_add_16_imm(self.reg.sp);
                      self.reg.set_hl(v);                             3 },
            0xF9 => { self.reg.sp = self.reg.get_hl();                2 },
            0xFA => { let a = self.read_word();
                      self.reg.a = self.mem.read(a);                  4 },
            0xFB => { self.ime_ask = true;                            1 },
            0xFE => { let b = self.read_byte();
                      self.alu_cp(b);                                 2 },
            0xFF => { self.rst(0x38)                                    },
            code => panic!("Instruction {:#04x} is unknown!", code),
        }
    }

    pub fn cb_call(&mut self) -> u32 {
        let opcode = self.read_byte();
        match opcode {
            0x00 => { self.reg.b = self.alu_rlc(self.reg.b);  2 },
            0x01 => { self.reg.c = self.alu_rlc(self.reg.c);  2 },
            0x02 => { self.reg.d = self.alu_rlc(self.reg.d);  2 },
            0x03 => { self.reg.e = self.alu_rlc(self.reg.e);  2 },
            0x04 => { self.reg.h = self.alu_rlc(self.reg.h);  2 },
            0x05 => { self.reg.l = self.alu_rlc(self.reg.l);  2 },
            0x06 => { let a = self.reg.get_hl();
                      let v = self.alu_rlc(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x07 => { self.reg.a = self.alu_rlc(self.reg.a);  2 },
            0x08 => { self.reg.b = self.alu_rrc(self.reg.b);  2 },
            0x09 => { self.reg.c = self.alu_rrc(self.reg.c);  2 },
            0x0A => { self.reg.d = self.alu_rrc(self.reg.d);  2 },
            0x0B => { self.reg.e = self.alu_rrc(self.reg.e);  2 },
            0x0C => { self.reg.h = self.alu_rrc(self.reg.h);  2 },
            0x0D => { self.reg.l = self.alu_rrc(self.reg.l);  2 },
            0x0E => { let a = self.reg.get_hl();
                      let v = self.alu_rrc(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x0F => { self.reg.a = self.alu_rrc(self.reg.a);  2 },
            0x10 => { self.reg.b = self.alu_rl(self.reg.b);   2 },
            0x11 => { self.reg.c = self.alu_rl(self.reg.c);   2 },
            0x12 => { self.reg.d = self.alu_rl(self.reg.d);   2 },
            0x13 => { self.reg.e = self.alu_rl(self.reg.e);   2 },
            0x14 => { self.reg.h = self.alu_rl(self.reg.h);   2 },
            0x15 => { self.reg.l = self.alu_rl(self.reg.l);   2 },
            0x16 => { let a = self.reg.get_hl();
                      let v = self.alu_rl(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x17 => { self.reg.a = self.alu_rl(self.reg.a);   2 },
            0x18 => { self.reg.b = self.alu_rr(self.reg.b);   2 },
            0x19 => { self.reg.c = self.alu_rr(self.reg.c);   2 },
            0x1A => { self.reg.d = self.alu_rr(self.reg.d);   2 },
            0x1B => { self.reg.e = self.alu_rr(self.reg.e);   2 },
            0x1C => { self.reg.h = self.alu_rr(self.reg.h);   2 },
            0x1D => { self.reg.l = self.alu_rr(self.reg.l);   2 },
            0x1E => { let a = self.reg.get_hl();
                      let v = self.alu_rr(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x1F => { self.reg.a = self.alu_rr(self.reg.a);   2 },
            0x20 => { self.reg.b = self.alu_sla(self.reg.b);  2 },
            0x21 => { self.reg.c = self.alu_sla(self.reg.c);  2 },
            0x22 => { self.reg.d = self.alu_sla(self.reg.d);  2 },
            0x23 => { self.reg.e = self.alu_sla(self.reg.e);  2 },
            0x24 => { self.reg.h = self.alu_sla(self.reg.h);  2 },
            0x25 => { self.reg.l = self.alu_sla(self.reg.l);  2 },
            0x26 => { let a = self.reg.get_hl();
                      let v = self.alu_sla(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x27 => { self.reg.a = self.alu_sla(self.reg.a);  2 },
            0x28 => { self.reg.b = self.alu_sra(self.reg.b);  2 },
            0x29 => { self.reg.c = self.alu_sra(self.reg.c);  2 },
            0x2A => { self.reg.d = self.alu_sra(self.reg.d);  2 },
            0x2B => { self.reg.e = self.alu_sra(self.reg.e);  2 },
            0x2C => { self.reg.h = self.alu_sra(self.reg.h);  2 },
            0x2D => { self.reg.l = self.alu_sra(self.reg.l);  2 },
            0x2E => { let a = self.reg.get_hl();
                      let v = self.alu_sra(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x2F => { self.reg.a = self.alu_sra(self.reg.a);  2 },
            0x30 => { self.reg.b = self.alu_swap(self.reg.b); 2 },
            0x31 => { self.reg.c = self.alu_swap(self.reg.c); 2 },
            0x32 => { self.reg.d = self.alu_swap(self.reg.d); 2 },
            0x33 => { self.reg.e = self.alu_swap(self.reg.e); 2 },
            0x34 => { self.reg.h = self.alu_swap(self.reg.h); 2 },
            0x35 => { self.reg.l = self.alu_swap(self.reg.l); 2 },
            0x36 => { let a = self.reg.get_hl();
                      let v = self.alu_swap(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x37 => { self.reg.a = self.alu_swap(self.reg.a); 2 },
            0x38 => { self.reg.b = self.alu_srl(self.reg.b);  2 },
            0x39 => { self.reg.c = self.alu_srl(self.reg.c);  2 },
            0x3A => { self.reg.d = self.alu_srl(self.reg.d);  2 },
            0x3B => { self.reg.e = self.alu_srl(self.reg.e);  2 },
            0x3C => { self.reg.h = self.alu_srl(self.reg.h);  2 },
            0x3D => { self.reg.l = self.alu_srl(self.reg.l);  2 },
            0x3E => { let a = self.reg.get_hl();
                      let v = self.alu_srl(self.mem.read(a));
                      self.mem.write(a, v);                   4 },
            0x3F => { self.reg.a = self.alu_srl(self.reg.a);  2 },
            0x40 => { self.alu_bit(self.reg.b, 0);       2 },
            0x41 => { self.alu_bit(self.reg.c, 0);       2 },
            0x42 => { self.alu_bit(self.reg.d, 0);       2 },
            0x43 => { self.alu_bit(self.reg.e, 0);       2 },
            0x44 => { self.alu_bit(self.reg.h, 0);       2 },
            0x45 => { self.alu_bit(self.reg.l, 0);       2 },
            0x46 => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 0); 3 },
            0x47 => { self.alu_bit(self.reg.a, 0);       2 },
            0x48 => { self.alu_bit(self.reg.b, 1);       2 },
            0x49 => { self.alu_bit(self.reg.c, 1);       2 },
            0x4A => { self.alu_bit(self.reg.d, 1);       2 },
            0x4B => { self.alu_bit(self.reg.e, 1);       2 },
            0x4C => { self.alu_bit(self.reg.h, 1);       2 },
            0x4D => { self.alu_bit(self.reg.l, 1);       2 },
            0x4E => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 1); 3 },
            0x4F => { self.alu_bit(self.reg.a, 1);       2 },
            0x50 => { self.alu_bit(self.reg.b, 2);       2 },
            0x51 => { self.alu_bit(self.reg.c, 2);       2 },
            0x52 => { self.alu_bit(self.reg.d, 2);       2 },
            0x53 => { self.alu_bit(self.reg.e, 2);       2 },
            0x54 => { self.alu_bit(self.reg.h, 2);       2 },
            0x55 => { self.alu_bit(self.reg.l, 2);       2 },
            0x56 => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 2); 3 },
            0x57 => { self.alu_bit(self.reg.a, 2);       2 },
            0x58 => { self.alu_bit(self.reg.b, 3);       2 },
            0x59 => { self.alu_bit(self.reg.c, 3);       2 },
            0x5A => { self.alu_bit(self.reg.d, 3);       2 },
            0x5B => { self.alu_bit(self.reg.e, 3);       2 },
            0x5C => { self.alu_bit(self.reg.h, 3);       2 },
            0x5D => { self.alu_bit(self.reg.l, 3);       2 },
            0x5E => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 3); 3 },
            0x5F => { self.alu_bit(self.reg.a, 3);       2 },
            0x60 => { self.alu_bit(self.reg.b, 4);       2 },
            0x61 => { self.alu_bit(self.reg.c, 4);       2 },
            0x62 => { self.alu_bit(self.reg.d, 4);       2 },
            0x63 => { self.alu_bit(self.reg.e, 4);       2 },
            0x64 => { self.alu_bit(self.reg.h, 4);       2 },
            0x65 => { self.alu_bit(self.reg.l, 4);       2 },
            0x66 => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 4); 3 },
            0x67 => { self.alu_bit(self.reg.a, 4);       2 },
            0x68 => { self.alu_bit(self.reg.b, 5);       2 },
            0x69 => { self.alu_bit(self.reg.c, 5);       2 },
            0x6A => { self.alu_bit(self.reg.d, 5);       2 },
            0x6B => { self.alu_bit(self.reg.e, 5);       2 },
            0x6C => { self.alu_bit(self.reg.h, 5);       2 },
            0x6D => { self.alu_bit(self.reg.l, 5);       2 },
            0x6E => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 5); 3 },
            0x6F => { self.alu_bit(self.reg.a, 5);       2 },
            0x70 => { self.alu_bit(self.reg.b, 6);       2 },
            0x71 => { self.alu_bit(self.reg.c, 6);       2 },
            0x72 => { self.alu_bit(self.reg.d, 6);       2 },
            0x73 => { self.alu_bit(self.reg.e, 6);       2 },
            0x74 => { self.alu_bit(self.reg.h, 6);       2 },
            0x75 => { self.alu_bit(self.reg.l, 6);       2 },
            0x76 => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 6); 3 },
            0x77 => { self.alu_bit(self.reg.a, 6);       2 },
            0x78 => { self.alu_bit(self.reg.b, 7);       2 },
            0x79 => { self.alu_bit(self.reg.c, 7);       2 },
            0x7A => { self.alu_bit(self.reg.d, 7);       2 },
            0x7B => { self.alu_bit(self.reg.e, 7);       2 },
            0x7C => { self.alu_bit(self.reg.h, 7);       2 },
            0x7D => { self.alu_bit(self.reg.l, 7);       2 },
            0x7E => { let a = self.reg.get_hl();
                      self.alu_bit(self.mem.read(a), 7); 3 },
            0x7F => { self.alu_bit(self.reg.a, 7);       2 },
            0x80 => { self.reg.b = self.alu_res(self.reg.b, 0);       2 },
            0x81 => { self.reg.c = self.alu_res(self.reg.c, 0);       2 },
            0x82 => { self.reg.d = self.alu_res(self.reg.d, 0);       2 },
            0x83 => { self.reg.e = self.alu_res(self.reg.e, 0);       2 },
            0x84 => { self.reg.h = self.alu_res(self.reg.h, 0);       2 },
            0x85 => { self.reg.l = self.alu_res(self.reg.l, 0);       2 },
            0x86 => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 0);
                      self.mem.write(a, v);                           4 },
            0x87 => { self.reg.a = self.alu_res(self.reg.a, 0);       2 },
            0x88 => { self.reg.b = self.alu_res(self.reg.b, 1);       2 },
            0x89 => { self.reg.c = self.alu_res(self.reg.c, 1);       2 },
            0x8A => { self.reg.d = self.alu_res(self.reg.d, 1);       2 },
            0x8B => { self.reg.e = self.alu_res(self.reg.e, 1);       2 },
            0x8C => { self.reg.h = self.alu_res(self.reg.h, 1);       2 },
            0x8D => { self.reg.l = self.alu_res(self.reg.l, 1);       2 },
            0x8E => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 1);
                      self.mem.write(a, v);                           4 },
            0x8F => { self.reg.a = self.alu_res(self.reg.a, 1);       2 },
            0x90 => { self.reg.b = self.alu_res(self.reg.b, 2);       2 },
            0x91 => { self.reg.c = self.alu_res(self.reg.c, 2);       2 },
            0x92 => { self.reg.d = self.alu_res(self.reg.d, 2);       2 },
            0x93 => { self.reg.e = self.alu_res(self.reg.e, 2);       2 },
            0x94 => { self.reg.h = self.alu_res(self.reg.h, 2);       2 },
            0x95 => { self.reg.l = self.alu_res(self.reg.l, 2);       2 },
            0x96 => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 2);
                      self.mem.write(a, v);                           4 },
            0x97 => { self.reg.a = self.alu_res(self.reg.a, 2);       2 },
            0x98 => { self.reg.b = self.alu_res(self.reg.b, 3);       2 },
            0x99 => { self.reg.c = self.alu_res(self.reg.c, 3);       2 },
            0x9A => { self.reg.d = self.alu_res(self.reg.d, 3);       2 },
            0x9B => { self.reg.e = self.alu_res(self.reg.e, 3);       2 },
            0x9C => { self.reg.h = self.alu_res(self.reg.h, 3);       2 },
            0x9D => { self.reg.l = self.alu_res(self.reg.l, 3);       2 },
            0x9E => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 3);
                      self.mem.write(a, v);                           4 },
            0x9F => { self.reg.a = self.alu_res(self.reg.a, 3);       2 },
            0xA0 => { self.reg.b = self.alu_res(self.reg.b, 4);       2 },
            0xA1 => { self.reg.c = self.alu_res(self.reg.c, 4);       2 },
            0xA2 => { self.reg.d = self.alu_res(self.reg.d, 4);       2 },
            0xA3 => { self.reg.e = self.alu_res(self.reg.e, 4);       2 },
            0xA4 => { self.reg.h = self.alu_res(self.reg.h, 4);       2 },
            0xA5 => { self.reg.l = self.alu_res(self.reg.l, 4);       2 },
            0xA6 => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 4);
                      self.mem.write(a, v);                           4 },
            0xA7 => { self.reg.a = self.alu_res(self.reg.a, 4);       2 },
            0xA8 => { self.reg.b = self.alu_res(self.reg.b, 5);       2 },
            0xA9 => { self.reg.c = self.alu_res(self.reg.c, 5);       2 },
            0xAA => { self.reg.d = self.alu_res(self.reg.d, 5);       2 },
            0xAB => { self.reg.e = self.alu_res(self.reg.e, 5);       2 },
            0xAC => { self.reg.h = self.alu_res(self.reg.h, 5);       2 },
            0xAD => { self.reg.l = self.alu_res(self.reg.l, 5);       2 },
            0xAE => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 5);
                      self.mem.write(a, v);                           4 },
            0xAF => { self.reg.a = self.alu_res(self.reg.a, 5);       2 },
            0xB0 => { self.reg.b = self.alu_res(self.reg.b, 6);       2 },
            0xB1 => { self.reg.c = self.alu_res(self.reg.c, 6);       2 },
            0xB2 => { self.reg.d = self.alu_res(self.reg.d, 6);       2 },
            0xB3 => { self.reg.e = self.alu_res(self.reg.e, 6);       2 },
            0xB4 => { self.reg.h = self.alu_res(self.reg.h, 6);       2 },
            0xB5 => { self.reg.l = self.alu_res(self.reg.l, 6);       2 },
            0xB6 => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 6);
                      self.mem.write(a, v);                           4 },
            0xB7 => { self.reg.a = self.alu_res(self.reg.a, 6);       2 },
            0xB8 => { self.reg.b = self.alu_res(self.reg.b, 7);       2 },
            0xB9 => { self.reg.c = self.alu_res(self.reg.c, 7);       2 },
            0xBA => { self.reg.d = self.alu_res(self.reg.d, 7);       2 },
            0xBB => { self.reg.e = self.alu_res(self.reg.e, 7);       2 },
            0xBC => { self.reg.h = self.alu_res(self.reg.h, 7);       2 },
            0xBD => { self.reg.l = self.alu_res(self.reg.l, 7);       2 },
            0xBE => { let a = self.reg.get_hl();
                      let v = self.alu_res(self.mem.read(a), 7);
                      self.mem.write(a, v);                           4 },
            0xBF => { self.reg.a = self.alu_res(self.reg.a, 7);       2 },
            0xC0 => { self.reg.b = self.alu_set(self.reg.b, 0); 2 },
            0xC1 => { self.reg.c = self.alu_set(self.reg.c, 0); 2 },
            0xC2 => { self.reg.d = self.alu_set(self.reg.d, 0); 2 },
            0xC3 => { self.reg.e = self.alu_set(self.reg.e, 0); 2 },
            0xC4 => { self.reg.h = self.alu_set(self.reg.h, 0); 2 },
            0xC5 => { self.reg.l = self.alu_set(self.reg.l, 0); 2 },
            0xC6 => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 0);
                      self.mem.write(a, v);                     4 },
            0xC7 => { self.reg.a = self.alu_set(self.reg.a, 0); 2 },
            0xC8 => { self.reg.b = self.alu_set(self.reg.b, 1); 2 },
            0xC9 => { self.reg.c = self.alu_set(self.reg.c, 1); 2 },
            0xCA => { self.reg.d = self.alu_set(self.reg.d, 1); 2 },
            0xCB => { self.reg.e = self.alu_set(self.reg.e, 1); 2 },
            0xCC => { self.reg.h = self.alu_set(self.reg.h, 1); 2 },
            0xCD => { self.reg.l = self.alu_set(self.reg.l, 1); 2 },
            0xCE => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 1);
                      self.mem.write(a, v);                     4 },
            0xCF => { self.reg.a = self.alu_set(self.reg.a, 1); 2 },
            0xD0 => { self.reg.b = self.alu_set(self.reg.b, 2); 2 },
            0xD1 => { self.reg.c = self.alu_set(self.reg.c, 2); 2 },
            0xD2 => { self.reg.d = self.alu_set(self.reg.d, 2); 2 },
            0xD3 => { self.reg.e = self.alu_set(self.reg.e, 2); 2 },
            0xD4 => { self.reg.h = self.alu_set(self.reg.h, 2); 2 },
            0xD5 => { self.reg.l = self.alu_set(self.reg.l, 2); 2 },
            0xD6 => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 2);
                      self.mem.write(a, v);                     4 },
            0xD7 => { self.reg.a = self.alu_set(self.reg.a, 2); 2 },
            0xD8 => { self.reg.b = self.alu_set(self.reg.b, 3); 2 },
            0xD9 => { self.reg.c = self.alu_set(self.reg.c, 3); 2 },
            0xDA => { self.reg.d = self.alu_set(self.reg.d, 3); 2 },
            0xDB => { self.reg.e = self.alu_set(self.reg.e, 3); 2 },
            0xDC => { self.reg.h = self.alu_set(self.reg.h, 3); 2 },
            0xDD => { self.reg.l = self.alu_set(self.reg.l, 3); 2 },
            0xDE => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 3);
                      self.mem.write(a, v);                     4 },
            0xDF => { self.reg.a = self.alu_set(self.reg.a, 3); 2 },
            0xE0 => { self.reg.b = self.alu_set(self.reg.b, 4); 2 },
            0xE1 => { self.reg.c = self.alu_set(self.reg.c, 4); 2 },
            0xE2 => { self.reg.d = self.alu_set(self.reg.d, 4); 2 },
            0xE3 => { self.reg.e = self.alu_set(self.reg.e, 4); 2 },
            0xE4 => { self.reg.h = self.alu_set(self.reg.h, 4); 2 },
            0xE5 => { self.reg.l = self.alu_set(self.reg.l, 4); 2 },
            0xE6 => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 4);
                      self.mem.write(a, v);                     4 },
            0xE7 => { self.reg.a = self.alu_set(self.reg.a, 4); 2 },
            0xE8 => { self.reg.b = self.alu_set(self.reg.b, 5); 2 },
            0xE9 => { self.reg.c = self.alu_set(self.reg.c, 5); 2 },
            0xEA => { self.reg.d = self.alu_set(self.reg.d, 5); 2 },
            0xEB => { self.reg.e = self.alu_set(self.reg.e, 5); 2 },
            0xEC => { self.reg.h = self.alu_set(self.reg.h, 5); 2 },
            0xED => { self.reg.l = self.alu_set(self.reg.l, 5); 2 },
            0xEE => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 5);
                      self.mem.write(a, v);                     4 },
            0xEF => { self.reg.a = self.alu_set(self.reg.a, 5); 2 },
            0xF0 => { self.reg.b = self.alu_set(self.reg.b, 6); 2 },
            0xF1 => { self.reg.c = self.alu_set(self.reg.c, 6); 2 },
            0xF2 => { self.reg.d = self.alu_set(self.reg.d, 6); 2 },
            0xF3 => { self.reg.e = self.alu_set(self.reg.e, 6); 2 },
            0xF4 => { self.reg.h = self.alu_set(self.reg.h, 6); 2 },
            0xF5 => { self.reg.l = self.alu_set(self.reg.l, 6); 2 },
            0xF6 => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 6);
                      self.mem.write(a, v);                     4 },
            0xF7 => { self.reg.a = self.alu_set(self.reg.a, 6); 2 },
            0xF8 => { self.reg.b = self.alu_set(self.reg.b, 7); 2 },
            0xF9 => { self.reg.c = self.alu_set(self.reg.c, 7); 2 },
            0xFA => { self.reg.d = self.alu_set(self.reg.d, 7); 2 },
            0xFB => { self.reg.e = self.alu_set(self.reg.e, 7); 2 },
            0xFC => { self.reg.h = self.alu_set(self.reg.h, 7); 2 },
            0xFD => { self.reg.l = self.alu_set(self.reg.l, 7); 2 },
            0xFE => { let a = self.reg.get_hl();
                      let mut v = self.mem.read(a);
                      v = self.alu_set(v, 7);
                      self.mem.write(a, v);                     4 },
            0xFF => { self.reg.a = self.alu_set(self.reg.a, 7); 2 },
            // code => panic!("CB Instruction {:#04x} is unknown!", code)
        }
    }

    fn jr(&mut self, cond: bool) -> u32 {
        let byte = self.read_byte() as i8;
        if cond {
            self.reg.pc = ((self.reg.pc as u32 as i32) + byte as i32) as u16;
            3
        } else {
            2
        }
    }

    fn jp(&mut self, cond: bool) -> u32 {
        let word = self.read_word();
        if cond {
            self.reg.pc = word;
            4
        } else {
            3
        }
    }

    fn call(&mut self, cond: bool) -> u32 {
        let word = self.read_word();
        if cond {
            self.push(self.reg.pc);
            self.reg.pc = word;
            6
        } else {
            3
        }
    }

    fn ret(&mut self, cond: bool) -> u32 {
        if cond {
            self.reg.pc = self.pop();
            5
        } else {
            2
        }
    }

    fn rst(&mut self, a: u16) -> u32 {
        self.push(self.reg.pc);
        self.reg.pc = a;
        4
    }

    fn alu_add(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a.wrapping_add(x);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) > u16::from(u8::MAX));
        self.reg.set_flag(Flags::H, (a & 0x0F) + (x & 0x0F) > 0x0F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.a = r;
    }

    fn alu_adc(&mut self, x: u8) {
        let a = self.reg.a;
        let c = u8::from(self.reg.get_flag(Flags::C));
        let r = a.wrapping_add(x).wrapping_add(c);
        self.reg.set_flag(Flags::C, u16::from(a) + u16::from(x) + u16::from(c) > u16::from(u8::MAX));
        self.reg.set_flag(Flags::H, (a & 0x0F) + (x & 0x0F)  + (c & 0x0F) > 0x0F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.a = r;
    }

    fn alu_add_16(&mut self, x: u16) {
        let a = self.reg.get_hl();
        let r = a.wrapping_add(x);
        self.reg.set_flag(Flags::C, u32::from(a) + u32::from(x) > u32::from(u16::MAX));
        self.reg.set_flag(Flags::H, (a & 0x0FFF) + (x & 0x0FFF) > 0x0FFF);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_hl(r);
    }

    fn alu_add_16_imm(&mut self, a: u16) -> u16 {
        let b = self.read_byte() as i8 as i16 as u16;
        self.reg.set_flag(Flags::C, (a & 0x00FF) + (b & 0x00FF) > 0x00FF);
        self.reg.set_flag(Flags::H, (a & 0x000F) + (b & 0x000F) > 0x000F);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, false);
        a.wrapping_add(b)
    }

    fn alu_sub(&mut self, x: u8) {
        let a = self.reg.a;
        let r = a.wrapping_sub(x);
        self.reg.set_flag(Flags::C, u16::from(a) < u16::from(x));
        self.reg.set_flag(Flags::H, (a & 0xF) < (x & 0xF));
        self.reg.set_flag(Flags::N, true);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.a = r;
    }

    fn alu_sbc(&mut self, x: u8) {
        let a = self.reg.a;
        let c = u8::from(self.reg.get_flag(Flags::C));
        let r = a.wrapping_sub(x).wrapping_sub(c);
        self.reg.set_flag(Flags::C, u16::from(a) < u16::from(x) + u16::from(c));
        self.reg.set_flag(Flags::H, (a & 0xF) < (x & 0xF) + c);
        self.reg.set_flag(Flags::N, true);
        self.reg.set_flag(Flags::Z, r == 0);
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

    fn alu_inc(&mut self, x: u8) -> u8 {
        let r = x.wrapping_add(1);
        self.reg.set_flag(Flags::H, (x & 0b0000_1111) + 0b0000_0001 > 0b0000_1111);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_dec(&mut self, x: u8) -> u8 {
        let r = x.wrapping_sub(1);
        self.reg.set_flag(Flags::H, x.trailing_zeros() >= 4);
        self.reg.set_flag(Flags::N, true);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_bit(&mut self, a: u8, b: u8) {
        let r = a & (1 << b) == 0x00;
        self.reg.set_flag(Flags::H, true);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r);
    }

    fn alu_set(&mut self, a: u8, b: u8) -> u8 {
        a | (1 << b)
    }

    fn alu_res(&mut self, a: u8, b: u8) -> u8 {
        a & !(1 << b)
    }

    fn alu_swap(&mut self, a: u8) -> u8 {
        self.reg.set_flag(Flags::C, false);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, a == 0);
        (a >> 4) | (a << 4)
    }

    fn alu_srl(&mut self, a: u8) -> u8 {
        let c = a & 0b0000_0001 == 0b0000_0001;
        let r = a >> 1;
        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_sla(&mut self, a: u8) -> u8 {
        let c = a & 0b1000_0000 == 0b1000_0000;
        let r = a << 1;
        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_sra(&mut self, a: u8) -> u8 {
        let c = a & 0b0000_0001 == 0b0000_0001;
        let r = (a >> 1) | (a & 0b1000_0000);
        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_rrc(&mut self, a: u8) -> u8 {
        let c = a & 0b0000_0001 == 0b0000_0001;
        let r = a.rotate_right(1);
        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_rr(&mut self, a: u8) -> u8 {
        let c = a & 0b0000_0001 == 0b0000_0001;
        let mut r = a >> 1;

        if self.reg.get_flag(Flags::C) {
            r = r | 0b1000_0000;
        }

        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_rlc(&mut self, a: u8) -> u8 {
        let c = a & 0b1000_0000 == 0b1000_0000;
        let r = a.rotate_left(1);
        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_rl(&mut self, a: u8) -> u8 {
        let c = (a & 0x80) >> 7 == 0x01;
        let r = (a << 1) + self.reg.get_flag(Flags::C) as u8;

        self.reg.set_flag(Flags::C, c);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::Z, r == 0);
        r
    }

    fn alu_daa(&mut self) {
        let mut a = self.reg.a;
        let mut adjust = if self.reg.get_flag(Flags::C) {
            0x60
        } else {
            0x00
        };

        if self.reg.get_flag(Flags::H) {
            adjust |= 0x06;
        };
        if !self.reg.get_flag(Flags::N) {
            if a & 0x0F > 0x09 {
                adjust |= 0x06;
            };
            if a > 0x99 {
                adjust |= 0x60;
            };
            a = a.wrapping_add(adjust);
        } else {
            a = a.wrapping_sub(adjust);
        }

        self.reg.set_flag(Flags::C, adjust >= 0x60);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::Z, a == 0x00);
        self.reg.a = a;
    }

    fn alu_cpl(&mut self) {
        self.reg.a = !self.reg.a;
        self.reg.set_flag(Flags::H, true);
        self.reg.set_flag(Flags::N, true);
    }

    fn alu_ccf(&mut self) {
        let v = !self.reg.get_flag(Flags::C);
        self.reg.set_flag(Flags::C, v);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
    }

    fn alu_scf(&mut self) {
        self.reg.set_flag(Flags::C, true);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::N, false);
    }
}