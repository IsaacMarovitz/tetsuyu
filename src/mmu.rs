use bitflags::bitflags;
use crate::sound::apu::APU;
use crate::joypad::Joypad;
use crate::mbc::mode::{MBC, MBCMode};
use crate::mbc::rom_only::ROMOnly;
use crate::mbc::mbc1::MBC1;
use crate::mbc::mbc2::MBC2;
use crate::mbc::mbc3::MBC3;
use crate::mbc::mbc5::MBC5;
use crate::memory::Memory;
use crate::ppu::PPU;
use crate::timer::Timer;
use crate::mode::GBMode;
use crate::serial::Serial;

pub struct MMU {
    mbc: Box<dyn MBC+'static>,
    pub ppu: PPU,
    apu: APU,
    serial: Serial,
    timer: Timer,
    pub joypad: Joypad,
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
    pub fn new(mode: GBMode,  mbc_mode: MBCMode, print_serial: bool, rom: Vec<u8>) -> Self {
        let mbc: Box<dyn MBC> = match mbc_mode {
            MBCMode::RomOnly => Box::new(ROMOnly::new(rom)),
            MBCMode::MBC1 => Box::new(MBC1::new(rom)),
            MBCMode::MBC2 => Box::new(MBC2::new(rom)),
            MBCMode::MBC3 => Box::new(MBC3::new(rom)),
            MBCMode::MBC5 => Box::new(MBC5::new(rom)),
            v => panic!("Unsupported MBC type! {:}", v)
        };

        Self {
            mbc: mbc,
            apu: APU::new(),
            ppu: PPU::new(mode),
            serial: Serial::new(print_serial),
            joypad: Joypad::new(),
            timer: Timer::new(),
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            intf: Interrupts::empty(),
            inte: Interrupts::empty(),
            wram_bank: 0x01
        }
    }

    pub fn cycle(&mut self, cycles: u32) -> bool {
        self.timer.cycle(cycles);
        self.intf |= self.timer.interrupts;
        self.timer.interrupts = Interrupts::empty();

        self.intf |= self.joypad.interrupts;
        self.joypad.interrupts = Interrupts::empty();

        let did_draw = self.ppu.cycle(cycles);
        self.intf |= self.ppu.interrupts;
        self.ppu.interrupts = Interrupts::empty();

        self.apu.cycle(self.timer.div);

        self.intf |= self.serial.interrupts;
        self.serial.interrupts = Interrupts::empty();

        did_draw
    }

    fn oamdma(&mut self, value: u8) {
        let base = (value as u16) << 8;
        for i in 0 .. 0xA0 {
            let b = self.read_word(base + i);
            self.write_word(0xFE00 + i, b);
        }
    }
}

impl Memory for MMU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x7FFF => self.mbc.read(a),
            0x8000..=0x9FFF => self.ppu.read(a),
            0xA000..=0xBFFF => self.mbc.read(a),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000],
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank],
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000],
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank],
            0xFE00..=0xFE9F => self.ppu.read(a),
            0xFF40..=0xFF4F => self.ppu.read(a),
            0xFF68..=0xFF6B => self.ppu.read(a),
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80],
            0xFF00 => self.joypad.read(a),
            0xFF01..=0xFF02 => self.serial.read(a),
            0xFF04..=0xFF07 => self.timer.read(a),
            0xFF10..=0xFF3F => self.apu.read(a),
            0xFF0F => self.intf.bits(),
            0xFF70 => self.wram_bank as u8,
            0xFEA0..=0xFEFF => 0xFF,
            0xFFFF => self.inte.bits(),
            _ => panic!("Read to unsupported address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x7FFF => self.mbc.write(a, v),
            0x8000..=0x9FFF => self.ppu.write(a, v),
            0xA000..=0xBFFF => self.mbc.write(a, v),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000] = v,
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank] = v,
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000] = v,
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank] = v,
            0xFE00..=0xFE9F => self.ppu.write(a, v),
            0xFF46 => self.oamdma(v),
            0xFF40..=0xFF4F => self.ppu.write(a, v),
            0xFF68..=0xFF6B => self.ppu.write(a, v),
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80] = v,
            0xFF00 => self.joypad.write(a, v),
            0xFF01..=0xFF02 => self.serial.write(a, v),
            0xFF04..=0xFF07 => self.timer.write(a, v),
            0xFF10..=0xFF3F => self.apu.write(a, v),
            0xFF0F => self.intf = Interrupts::from_bits_truncate(v),
            0xFF50..=0xFF5F => {},
            0xFF70 => self.wram_bank = match v & 0x07 { 0 => 1, n => n as usize },
            0xFEA0..=0xFEFF => {},
            0xFF7F => {},
            0xFFFF => self.inte = Interrupts::from_bits_truncate(v),
            _ => panic!("Write to unsupported address ({:#06x})!", a),
        }
    }
}