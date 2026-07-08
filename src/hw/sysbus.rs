use super::bus::{BusDir, Chip, Pins, Ticked};
use super::interrupt::Interrupts;
use crate::components::joypad::{Joypad, JoypadButton};
use crate::components::memory::Memory;
use crate::components::mode::GBMode;
use crate::components::serial::Serial;
use crate::config::Config;
use crate::mbc::header::Header;
use crate::mbc::mode::{MBC, MBCMode};
use crate::mbc::prelude::*;

pub struct SystemBus {
    mbc: Box<dyn MBC + 'static>,
    serial: Serial,
    joypad: Joypad,
    wram: [u8; 0x8000],
    hram: [u8; 0x7F],
    wram_bank: usize,
    boot_rom: [u8; 0x900],
    boot_rom_enabled: bool,
    boot_just_disabled: bool,
    mode: GBMode,
    double_speed: bool,
    key1_armed: bool,
    rp: u8,
}

impl SystemBus {
    pub fn new(rom: Vec<u8>, header: Header, config: &Config, boot_rom: [u8; 0x900]) -> Self {
        let mbc: Box<dyn MBC> = match header.cart_type.get_mbc() {
            MBCMode::RomOnly => Box::new(ROMOnly::new(rom)),
            MBCMode::MBC1 => Box::new(MBC1::new(rom)),
            MBCMode::MBC2 => Box::new(MBC2::new(rom)),
            MBCMode::MBC3 => Box::new(MBC3::new(rom)),
            MBCMode::MBC5 => Box::new(MBC5::new(rom)),
            v => panic!("Unsupported MBC type! {:}", v),
        };

        Self {
            mbc,
            serial: Serial::new(config.print_serial, config.mode),
            joypad: Joypad::new(),
            wram: [0; 0x8000],
            hram: [0; 0x7F],
            wram_bank: 0x01,
            boot_rom,
            boot_rom_enabled: true,
            boot_just_disabled: false,
            mode: config.mode,
            double_speed: false,
            key1_armed: false,
            rp: 0,
        }
    }

    pub fn double_speed(&self) -> bool {
        self.double_speed
    }

    /// Executed when the CPU runs STOP; flips speed if a switch is armed.
    pub fn try_speed_switch(&mut self) {
        if self.key1_armed {
            self.double_speed = !self.double_speed;
            self.key1_armed = false;
        }
    }

    /// True once after a write to 0xFF50; the motherboard forwards it to the PPU.
    pub fn take_boot_disabled(&mut self) -> bool {
        std::mem::take(&mut self.boot_just_disabled)
    }

    pub fn joypad_down(&mut self, b: JoypadButton) {
        self.joypad.down(b);
    }

    pub fn joypad_up(&mut self, b: JoypadButton) {
        self.joypad.up(b);
    }

    pub fn peek(&self, a: u16) -> u8 {
        self.do_read(a)
    }

    pub fn disable_boot(&mut self) {
        self.boot_rom_enabled = false;
    }

    /// Bytes the program has transmitted over the serial port.
    pub fn serial_output(&self) -> &[u8] {
        self.serial.output()
    }

    fn owns(a: u16) -> bool {
        matches!(a,
            0x0000..=0x7FFF
            | 0xA000..=0xBFFF
            | 0xC000..=0xFDFF
            | 0xFEA0..=0xFEFF
            | 0xFF00
            | 0xFF01..=0xFF02
            | 0xFF4D
            | 0xFF50
            | 0xFF56
            | 0xFF70
            | 0xFF7F
            | 0xFF80..=0xFFFE)
    }

    fn do_read(&self, a: u16) -> u8 {
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
                if self.mode != GBMode::DMG && self.boot_rom_enabled {
                    self.boot_rom[a as usize]
                } else {
                    self.mbc.read(a)
                }
            }
            0x0900..=0x7FFF => self.mbc.read(a),
            0xA000..=0xBFFF => self.mbc.read(a),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000],
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank.max(1)],
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000],
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank.max(1)],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00 => self.joypad.read(a),
            0xFF01..=0xFF02 => self.serial.read(a),
            0xFF4D => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    0x7E | if self.double_speed { 0x80 } else { 0x00 }
                        | if self.key1_armed { 0x01 } else { 0x00 }
                }
            }
            0xFF56 => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    self.rp | 0x3E
                }
            }
            0xFF70 => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    0xF8 | self.wram_bank as u8
                }
            }
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80],
            _ => 0xFF,
        }
    }

    fn do_write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x7FFF => self.mbc.write(a, v),
            0xA000..=0xBFFF => self.mbc.write(a, v),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000] = v,
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank.max(1)] = v,
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000] = v,
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank.max(1)] = v,
            0xFEA0..=0xFEFF => {}
            0xFF00 => self.joypad.write(a, v),
            0xFF01..=0xFF02 => self.serial.write(a, v),
            0xFF4D => self.key1_armed = (v & 0x01) != 0,
            0xFF50 => {
                self.boot_rom_enabled = false;
                self.boot_just_disabled = true;
            }
            0xFF56 => {
                if self.mode != GBMode::DMG {
                    self.rp = v & 0xC1;
                }
            }
            0xFF70 => {
                if self.mode != GBMode::DMG {
                    self.wram_bank = (v & 0x07) as usize;
                }
            }
            0xFF7F => {}
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80] = v,
            _ => {}
        }
    }
}

impl Chip for SystemBus {
    fn advance(&mut self, _base_dot: bool) -> Ticked {
        // Drain the untimed ports' interrupt requests.
        let mut bits = self.serial.interrupts.bits();
        self.serial.interrupts = Interrupts::empty();
        bits |= self.joypad.interrupts.bits();
        self.joypad.interrupts = Interrupts::empty();
        Ticked {
            irq: Interrupts::from_bits_truncate(bits),
            hblank_edge: false,
        }
    }

    fn bus(&mut self, pins: &mut Pins) -> Ticked {
        if pins.transfer && Self::owns(pins.address) {
            match pins.dir {
                BusDir::Read => pins.data = self.do_read(pins.address),
                BusDir::Write => self.do_write(pins.address, pins.data),
                BusDir::Idle => {}
            }
        }
        Ticked::default()
    }
}
