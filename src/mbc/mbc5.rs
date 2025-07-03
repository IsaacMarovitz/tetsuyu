use crate::components::memory::Memory;
use crate::mbc::mode::MBC;

pub struct MBC5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    rom_bank: usize,
    ram_bank: usize,
}

impl Memory for MBC5 {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3FFF => self.rom[a as usize],
            0x4000..=0x7FFF => self.rom[a as usize + self.rom_bank * 0x4000 - 0x4000],
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.ram[a as usize + self.ram_bank * 0x2000 - 0xA000]
                } else {
                    0x00
                }
            }
            _ => panic!("Read to unsupported MBC5 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x1FFF => self.ram_enabled = v & 0x0F == 0x0A,
            0x2000..=0x2FFF => self.rom_bank = (self.rom_bank & 0x100) | (v as usize),
            0x3000..=0x3FFF => {
                self.rom_bank = (self.rom_bank & 0x0ff) | (((v & 0x01) as usize) << 8)
            }
            0x4000..=0x5FFF => self.ram_bank = (v & 0x0f) as usize,
            // Unknown writes
            0x6000..=0x7FFF => {}
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.ram[a as usize + self.ram_bank * 0x2000 - 0xA000] = v;
                }
            }
            _ => panic!("Write to unsupported MBC5 address ({:#06x})!", a),
        }
    }
}

impl MBC for MBC5 {}

impl MBC5 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0x00; 131_072],
            ram_enabled: false,
            rom_bank: 0,
            ram_bank: 0,
        }
    }
}
