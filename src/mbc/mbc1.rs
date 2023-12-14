use crate::mbc::mode::MBC;
use crate::memory::Memory;

pub struct MBC1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    banking_mode: u8,
    secondary_bank: u8,
    rom_bank: u8,
    ram_bank: u8,
    big_rom: bool
}

// TODO: MBC1M Support

impl Memory for MBC1 {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3FFF => {
                if self.banking_mode == 0x01 && self.big_rom {
                    self.rom[a as usize + self.rom_bank as usize * 0x4000]
                } else {
                    self.rom[a as usize]
                }
            },
            0x4000..=0x7FFF => {
                let mut rom_bank = if self.rom_bank == 0x00 { 0x01 } else { self.rom_bank };
                rom_bank = (self.secondary_bank << 5) + rom_bank;

                // TODO: Set this on number of bits required to address all banks
                let rom_mask = 0b0001_1111;
                rom_bank &= rom_mask;

                self.rom[a as usize + (rom_bank - 1) as usize * 0x4000]
            },
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.ram[a as usize + self.ram_bank as usize * 0x4000]
                } else {
                    0xFF
                }
            }
            _ => panic!("Read to unsupported MBC1 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x1FFF => self.ram_enabled = v & 0xF == 0xA,
            0x2000..=0x3FFF => self.rom_bank = v,
            0x4000..=0x5FFF => {
                if !self.big_rom {
                    self.ram_bank = v;
                } else {
                    self.secondary_bank = v;
                }
            },
            0x6000..=0x7FFF => self.banking_mode = v & 0x01,
            _ => panic!("Write to unsupported MBC1 address ({:#06x})!", a),
        }
    }
}

impl MBC for MBC1 { }

impl MBC1 {
    pub fn new(rom: Vec<u8>) -> Self {
        let big_rom = rom.len() >= 1048576;
        let ram_size = if big_rom { 8192 } else { 32768 };
        let rom_size = if big_rom { 2097152 } else { 524288 };
        let mut padded_rom = vec![0x00; rom_size];
        padded_rom[0..rom.len()].copy_from_slice(rom.as_slice());

        Self {
            rom: padded_rom,
            ram: vec![0x00; ram_size],
            ram_enabled: false,
            banking_mode: 0x00,
            secondary_bank: 0x00,
            rom_bank: 0x00,
            ram_bank: 0x00,
            big_rom
        }
    }
}