use bitflags::bitflags;
use crate::ppu::PPU;
use crate::mode::GBMode;
use crate::serial::Serial;

pub struct MMU {
    rom: Vec<u8>,
    pub ppu: PPU,
    serial: Serial,
    wram: [u8; 0x8000],
    hram: [u8; 0x7F],
    intf: Interrupts,
    inte: Interrupts,
    wram_bank: usize,
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct Interrupts: u8 {
        const JOYPAD = 0b0001_0000;
        const SERIAL = 0b0000_1000;
        const TIMER = 0b0000_0100;
        const LCD = 0b0000_0010;
        const V_BLANK = 0b0000_0001;
    }
}

impl MMU {
    pub fn new(mode: GBMode, rom: Vec<u8>) -> Self {
        Self {
            rom,
            ppu: PPU::new(mode),
            serial: Serial::new(),
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            intf: Interrupts::empty(),
            inte: Interrupts::empty(),
            wram_bank: 0x01
        }
    }

    pub fn cycle(&mut self, cycles: u32) -> bool {
        let did_draw = self.ppu.cycle(cycles);
        self.intf |= self.ppu.interrupts;
        self.ppu.interrupts = Interrupts::empty();

        self.intf |= self.serial.interrupts;
        self.serial.interrupts = Interrupts::empty();

        did_draw
    }

    pub fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x7FFF => self.rom[a as usize],
            0x8000..=0x9FFF => self.ppu.read(a),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000],
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank],
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000],
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank],
            0xFF40..=0xFF4F => self.ppu.read(a),
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80],
            0xFF01..=0xFF02 => self.serial.read(a),
            0xFF0F => self.intf.bits(),
            0xFFFF => self.inte.bits(),
            _ => panic!("Read to unsupported address ({:#06x})!", a),
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x7FFF => self.rom[a as usize] = v,
            0x8000..=0x9FFF => self.ppu.write(a, v),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000] = v,
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank] = v,
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000] = v,
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank] = v,
            0xFF40..=0xFF4F => self.ppu.write(a, v),
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80] = v,
            // TODO: Joypad
            0xFF00 => {},
            0xFF01..=0xFF02 => self.serial.write(a, v),
            // TODO: Timer
            0xFF04..=0xFF07 => {},
            0xFF0F => self.intf = Interrupts::from_bits(v).unwrap(),
            0xFF24 => {},
            0xFF25 => {},
            0xFF26 => {},
            0xFFFF => self.inte = Interrupts::from_bits(v).unwrap(),
            _ => panic!("Write to unsupported address ({:#06x})!", a),
        }
    }

    pub fn read_word(&self, a: u16) -> u16 {
        (self.read(a) as u16) | ((self.read(a + 1) as u16) << 8)
    }

    pub fn write_word(&mut self, a: u16, v: u16) {
        self.write(a, (v & 0xFF) as u8);
        self.write(a + 1, (v >> 8) as u8);
    }
}