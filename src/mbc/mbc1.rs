use crate::components::memory::Memory;
use crate::mbc::mode::MBC;

pub struct MBC1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    bank_mode: BankMode,
    bank: u8,
}

// TODO: MBC1M Support

impl Memory for MBC1 {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3FFF => self.rom[a as usize],
            0x4000..=0x7FFF => self.rom[a as usize + self.rom_bank() * 0x4000 - 0x4000],
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.ram[a as usize + self.ram_bank() * 0x2000 - 0xA000]
                } else {
                    0x00
                }
            }
            _ => panic!("Read to unsupported MBC1 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x1FFF => self.ram_enabled = v & 0xF == 0xA,
            0x2000..=0x3FFF => {
                let n = match v & 0x1F {
                    0x00 => 0x01,
                    n => n,
                };
                self.bank = (self.bank & 0x60) | n;
            }
            0x4000..=0x5FFF => self.bank = self.bank & 0x9F | ((v & 0x03) << 5),
            0x6000..=0x7FFF => match v {
                0x00 => self.bank_mode = BankMode::ROM,
                0x01 => self.bank_mode = BankMode::RAM,
                n => panic!("Unknown bank mode ({:#04x})!", n),
            },
            0xA000..=0xBFFF => {
                let ram_bank = self.ram_bank();
                if self.ram_enabled {
                    self.ram[a as usize + ram_bank * 0x2000 - 0xA000] = v;
                }
            }
            _ => panic!("Write to unsupported MBC1 address ({:#06x})!", a),
        }
    }
}

impl MBC for MBC1 {}

impl MBC1 {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut padded_rom = vec![0x00; 2_097_152];
        padded_rom[0..rom.len()].copy_from_slice(rom.as_slice());

        Self {
            rom: padded_rom,
            ram: vec![0x00; 32_768],
            ram_enabled: false,
            bank_mode: BankMode::ROM,
            bank: 0x01,
        }
    }

    fn rom_bank(&self) -> usize {
        let n = match self.bank_mode {
            BankMode::ROM => self.bank & 0x7F,
            BankMode::RAM => self.bank & 0x1F,
        };
        n as usize
    }

    fn ram_bank(&self) -> usize {
        let n = match self.bank_mode {
            BankMode::ROM => 0x00,
            BankMode::RAM => (self.bank & 0x60) >> 5,
        };
        n as usize
    }
}

enum BankMode {
    ROM,
    RAM,
}
