use crate::mbc::mode::MBC;
use crate::components::memory::Memory;

pub struct MBC2 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    rom_bank: usize
}

impl Memory for MBC2 {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3FFF => self.rom[a as usize],
            0x4000..=0x7FFF => self.rom[a as usize + self.rom_bank * 0x4000 - 0x4000],
            0xA000..=0xA1FF => {
                if self.ram_enabled {
                    self.ram[(a - 0xA000) as usize]
                } else {
                    0x00
                }
            }
            _ => panic!("Read to unsupported MBC2 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        let v = v & 0x0F;
        match a {
            0x0000..=0x1FFF => {
                if a & 0x0100 == 0 {
                    self.ram_enabled = v == 0x0A;
                }
            },
            0x2000..=0x3FFF => {
                if a & 0x0100 != 0 {
                    self.rom_bank = v as usize;
                }
            },
            0xA000..=0xA1FF => {
                if self.ram_enabled {
                    self.ram[(a - 0xa000) as usize] = v
                }
            }
            _ => panic!("Write to unsupported MBC2 address ({:#06x})!", a),
        }
    }
}

impl MBC for MBC2 { }

impl MBC2 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0x00; 512],
            ram_enabled: false,
            rom_bank: 1
        }
    }
}

