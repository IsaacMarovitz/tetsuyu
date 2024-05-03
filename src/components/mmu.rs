use crate::components::prelude::*;
use crate::config::Config;
use crate::mbc::prelude::*;
use crate::sound::apu::APU;
use crate::Framebuffer;
use bitflags::bitflags;
use num_traits::FromPrimitive;
use crate::components::prelude::ppu::PPU;

pub struct MMU {
    mbc: Box<dyn MBC + 'static>,
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
    boot_rom: [u8; 0x900],
    boot_rom_enabled: bool,
    mode: GBMode
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
    pub fn new(rom: Vec<u8>,
               config: Config,
               boot_rom: [u8; 0x900],
               framebuffer: Framebuffer) -> Self {
        let cart_type: CartTypes = FromPrimitive::from_u8(rom[0x0147]).expect("Failed to get Cart Type!");
        let mbc_mode = match cart_type.get_mbc() {
            MBCMode::Unsupported => panic!("Unsupported Cart Type! {:}", cart_type),
            v => {
                println!("Cart Type: {:}, MBC Type: {:}", cart_type, v);
                v
            }
        };

        let mbc: Box<dyn MBC> = match mbc_mode {
            MBCMode::RomOnly => Box::new(ROMOnly::new(rom)),
            MBCMode::MBC1 => Box::new(MBC1::new(rom)),
            MBCMode::MBC2 => Box::new(MBC2::new(rom)),
            MBCMode::MBC3 => Box::new(MBC3::new(rom)),
            MBCMode::MBC5 => Box::new(MBC5::new(rom)),
            v => panic!("Unsupported MBC type! {:}", v),
        };

        Self {
            mbc: mbc,
            apu: APU::new(config.apu_config),
            ppu: PPU::new(config.clone(), framebuffer),
            serial: Serial::new(config.print_serial),
            joypad: Joypad::new(),
            timer: Timer::new(),
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            intf: Interrupts::empty(),
            inte: Interrupts::empty(),
            wram_bank: 0x01,
            boot_rom,
            boot_rom_enabled: true,
            mode: config.mode
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        self.timer.cycle(cycles);
        self.intf |= self.timer.interrupts;
        self.timer.interrupts = Interrupts::empty();

        self.intf |= self.joypad.interrupts;
        self.joypad.interrupts = Interrupts::empty();

        self.ppu.cycle(cycles);
        self.intf |= self.ppu.interrupts;
        self.ppu.interrupts = Interrupts::empty();

        self.apu.cycle(self.timer.div);

        self.intf |= self.serial.interrupts;
        self.serial.interrupts = Interrupts::empty();
    }

    fn oamdma(&mut self, value: u8) {
        let base = (value as u16) << 8;
        for i in 0..0xA0 {
            let b = self.read_word(base + i);
            self.write_word(0xFE00 + i, b);
        }
    }
}

impl Memory for MMU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x00FF => {
                if self.boot_rom_enabled {
                    self.boot_rom[a as usize]
                } else {
                    self.mbc.read(a)
                }
            }
            0x0100..=0x01FF => self.mbc.read(a),
            0x0200..=0x08FF => {
                if self.mode == GBMode::DMG {
                    self.mbc.read(a)
                } else {
                    if self.boot_rom_enabled {
                        self.boot_rom[a as usize]
                    } else {
                        self.mbc.read(a)
                    }
                }
            }
            0x0900..=0x7FFF => self.mbc.read(a),
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
            // TODO: RP
            0xFF56 => 0x00,
            0xFF51..=0xFF6F => self.ppu.read(a),
            0xFF70 => 0xF8 | self.wram_bank as u8,
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
            0xFF50 => self.boot_rom_enabled = false,
            // TODO: RP
            0xFF56 => { },
            0xFF51..=0xFF6F => self.ppu.write(a, v),
            0xFF70 => {
                self.wram_bank = match v & 0x07 {
                    0 => 1,
                    n => n as usize,
                }
            }
            0xFEA0..=0xFEFF => {}
            0xFF7F => {}
            0xFFFF => self.inte = Interrupts::from_bits_truncate(v),
            _ => panic!("Write to unsupported address ({:#06x})!", a),
        }
    }
}
