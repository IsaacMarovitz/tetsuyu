use std::time::SystemTime;
use crate::mbc::mode::MBC;
use crate::memory::Memory;

pub struct MBC3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rtc: RTC,
    ram_enabled: bool,
    rom_bank: usize,
    ram_bank: usize
}

impl Memory for MBC3 {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3FFF => self.rom[a as usize],
            0x4000..=0x7FFF => self.rom[a as usize + self.rom_bank * 0x4000 - 0x4000],
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    if self.ram_bank <= 0x03 {
                        self.ram[a as usize + self.ram_bank * 0x2000 - 0xA000]
                    } else {
                        self.rtc.read(self.ram_bank as u16)
                    }
                } else {
                    0x00
                }
            }
            _ => panic!("Read to unsupported MBC3 address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x1FFF => self.ram_enabled = v & 0x0F == 0x0A,
            0x2000..=0x3FFF => {
                let n = match v & 0x7F {
                    0x00 => 0x01,
                    n => n,
                };
                self.rom_bank = n as usize;
            },
            0x4000..=0x5FFF => self.ram_bank = (v & 0x0F) as usize,
            0x6000..=0x7FFF => {
                if v & 0x01 != 0 {
                    self.rtc.tick();
                }
            },
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    if self.ram_bank <= 0x03 {
                        self.ram[a as usize + self.ram_bank * 0x2000 - 0xA000] = v;
                    } else {
                        self.rtc.write(self.ram_bank as u16, v);
                    }
                }
            },
            _ => panic!("Write to unsupported MBC3 address ({:#06x})!", a),
        }
    }
}

impl MBC for MBC3 { }

impl MBC3 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: vec![0x00; 32_768],
            rtc: RTC::new(),
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0
        }
    }
}

struct RTC {
    s: u8,
    m: u8,
    h: u8,
    dl: u8,
    dh: u8
}

impl RTC {
    pub fn new() -> Self {
        Self {
            s: 0,
            m: 0,
            h: 0,
            dl: 0,
            dh: 0
        }
    }

    pub fn tick(&mut self) {
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.s = (d % 60) as u8;
        self.m = (d / 60 % 60) as u8;
        self.h = (d / 3600 % 24) as u8;
        let days = (d / 3600 / 24) as u16;
        self.dl = (days % 256) as u8;
        match days {
            0x0000..=0x00ff => {}
            0x0100..=0x01ff => {
                self.dh |= 0x01;
            }
            _ => {
                self.dh |= 0x01;
                self.dh |= 0x80;
            }
        }
    }
}

impl Memory for RTC {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x08 => self.s,
            0x09 => self.m,
            0x0A => self.h,
            0x0B => self.dl,
            0x0C => self.dh,
            _ => panic!("Read to unsupported RTC address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x08 => self.s = v,
            0x09 => self.m = v,
            0x0A => self.h = v,
            0x0B => self.dl = v,
            0x0C => self.dh = v,
            _ => panic!("Write to unsupported RTC address ({:#06x})!", a),
        }
    }
}