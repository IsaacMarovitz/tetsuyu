use crate::components::prelude::ppu::PPU;
use crate::components::prelude::*;
use crate::config::Config;
use crate::framebuffer::FramebufferWriter;
use crate::mbc::header::Header;
use crate::mbc::prelude::*;
use crate::sound::apu::APU;
use bitflags::bitflags;

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
    mode: GBMode,

    oam_dma_src: u16,
    oam_dma_progress: u16,
    oam_dma_timer: u32,
    oam_dma_active: bool,

    hdma_src: u16,
    hdma_dst: u16,
    hdma_len: u8,
    hdma_hblank: bool,

    double_speed: bool,
    key1_armed: bool,
    rp: u8,
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
    pub fn new(
        rom: Vec<u8>,
        header: Header,
        config: Config,
        boot_rom: [u8; 0x900],
        framebuffer: FramebufferWriter,
        rom_is_cgb: bool,
    ) -> Self {
        let mbc_mode = match header.cart_type.get_mbc() {
            MBCMode::Unsupported => panic!("Unsupported Cart Type! {:}", header.cart_type),
            v => v,
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
            mbc,
            apu: APU::new(config.apu_config, config.mode),
            ppu: PPU::new(config.clone(), framebuffer, rom_is_cgb),
            serial: Serial::new(config.print_serial, config.mode),
            joypad: Joypad::new(),
            timer: Timer::new(),
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            intf: Interrupts::empty(),
            inte: Interrupts::empty(),
            wram_bank: 0x01,
            boot_rom,
            boot_rom_enabled: true,
            mode: config.mode,

            oam_dma_src: 0,
            oam_dma_progress: 0,
            oam_dma_timer: 0,
            oam_dma_active: false,

            hdma_src: 0,
            hdma_dst: 0,
            hdma_len: 0,
            hdma_hblank: false,

            double_speed: false,
            key1_armed: false,
            rp: 0,
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        self.step_oam_dma(cycles);

        self.timer.cycle(cycles);
        self.intf |= self.timer.interrupts;
        self.timer.interrupts = Interrupts::empty();

        self.intf |= self.joypad.interrupts;
        self.joypad.interrupts = Interrupts::empty();

        self.ppu.cycle(cycles >> self.double_speed as u32);
        self.intf |= self.ppu.interrupts;
        self.ppu.interrupts = Interrupts::empty();

        if self.ppu.entered_hblank {
            self.ppu.entered_hblank = false;
            self.step_hdma();
        }

        self.apu.cycle(self.timer.div());

        self.intf |= self.serial.interrupts;
        self.serial.interrupts = Interrupts::empty();
    }

    fn start_oam_dma(&mut self, value: u8) {
        self.oam_dma_src = (value as u16) << 8;
        self.oam_dma_progress = 0;
        self.oam_dma_timer = 0;
        self.oam_dma_active = true;
    }

    fn step_oam_dma(&mut self, cycles: u32) {
        if !self.oam_dma_active {
            return;
        }

        self.oam_dma_timer += cycles;
        while self.oam_dma_timer >= 4 && self.oam_dma_progress < 0xA0 {
            self.oam_dma_timer -= 4;
            let byte = self.read(self.oam_dma_src + self.oam_dma_progress);
            self.ppu.write_oam(self.oam_dma_progress, byte);
            self.oam_dma_progress += 1;
        }

        if self.oam_dma_progress >= 0xA0 {
            self.oam_dma_active = false;
        }
    }

    fn start_hdma(&mut self, v: u8) {
        // Writing bit 7 = 0 while an HBlank transfer is active cancels it.
        if self.hdma_len != 0xFF && (v & 0x80) == 0 {
            self.hdma_len = 0xFF;
            return;
        }

        self.hdma_len = v & 0x7F;
        self.hdma_hblank = (v & 0x80) != 0;

        if !self.hdma_hblank {
            // GPDMA: copy everything at once, halting the CPU. The bus still
            // advances 8 M-cycles per 16-byte block (16 in double speed).
            let blocks = (self.hdma_len as u16) + 1;
            for _ in 0..blocks {
                self.hdma_copy_block();
                self.cycle(32 << self.double_speed as u32);
            }
            self.hdma_len = 0xFF;
        }
    }

    fn step_hdma(&mut self) {
        if self.hdma_len == 0xFF || !self.hdma_hblank {
            return;
        }
        self.hdma_copy_block();
    }

    fn hdma_copy_block(&mut self) {
        for _ in 0..0x10 {
            let byte = self.read(self.hdma_src);
            self.ppu.write(self.hdma_dst, byte);
            self.hdma_src = self.hdma_src.wrapping_add(1);
            self.hdma_dst = self.hdma_dst.wrapping_add(1);
        }
        if self.hdma_len != 0 {
            self.hdma_len -= 1;
        } else {
            self.hdma_len = 0xFF;
        }
    }

    pub fn speed_switch(&mut self) {
        if self.key1_armed {
            self.double_speed = !self.double_speed;
            self.key1_armed = false;
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
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank.max(1)],
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000],
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank.max(1)],
            0xFE00..=0xFE9F => self.ppu.read(a),
            0xFF4D => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    0x7E | if self.double_speed { 0x80 } else { 0x00 }
                        | if self.key1_armed { 0x01 } else { 0x00 }
                }
            }
            0xFF40..=0xFF4F => self.ppu.read(a),
            0xFF68..=0xFF6B => self.ppu.read(a),
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80],
            0xFF00 => self.joypad.read(a),
            0xFF01..=0xFF02 => self.serial.read(a),
            0xFF04..=0xFF07 => self.timer.read(a),
            0xFF10..=0xFF3F => self.apu.read(a),
            0xFF0F => self.intf.bits() | 0xE0,
            0xFF55 => self.hdma_len,
            0xFF56 => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    self.rp | 0x3E
                }
            }
            0xFF51..=0xFF6F => self.ppu.read(a),
            0xFF70 => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    0xF8 | self.wram_bank as u8
                }
            }
            0xFEA0..=0xFEFF => 0xFF,
            0xFFFF => self.inte.bits(),
            _ => 0xFF
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x7FFF => self.mbc.write(a, v),
            0x8000..=0x9FFF => self.ppu.write(a, v),
            0xA000..=0xBFFF => self.mbc.write(a, v),
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000] = v,
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank.max(1)] = v,
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000] = v,
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank.max(1)] = v,
            0xFE00..=0xFE9F => self.ppu.write(a, v),
            0xFF46 => self.start_oam_dma(v),
            0xFF4D => self.key1_armed = (v & 0x01) != 0,
            0xFF40..=0xFF4F => {
                if !(self.mode == GBMode::DMG && a == 0xFF4F) {
                    self.ppu.write(a, v);
                }
            }
            0xFF68..=0xFF6B => {
                if self.mode != GBMode::DMG {
                    self.ppu.write(a, v);
                }
            }
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80] = v,
            0xFF00 => self.joypad.write(a, v),
            0xFF01..=0xFF02 => self.serial.write(a, v),
            0xFF04..=0xFF07 => self.timer.write(a, v),
            0xFF10..=0xFF3F => self.apu.write(a, v),
            0xFF0F => self.intf = Interrupts::from_bits_truncate(v),
            0xFF50 => {
                self.boot_rom_enabled = false;
                self.ppu.clear_vram();
                self.ppu.disable_boot_rom();
            }
            0xFF51 => self.hdma_src = (self.hdma_src & 0x00FF) | ((v as u16) << 8),
            0xFF52 => self.hdma_src = (self.hdma_src & 0xFF00) | (v as u16 & 0xF0),
            0xFF53 => self.hdma_dst = 0x8000 | (self.hdma_dst & 0x00FF) | (((v as u16 & 0x1F) << 8)),
            0xFF54 => self.hdma_dst = (self.hdma_dst & 0xFF00) | (v as u16 & 0xF0),
            0xFF55 => self.start_hdma(v),
            0xFF56 => {
                if self.mode != GBMode::DMG {
                    self.rp = v & 0xC1;
                }
            }
            0xFF51..=0xFF6F => self.ppu.write(a, v),
            0xFF70 => {
                if self.mode != GBMode::DMG {
                    self.wram_bank = (v & 0x07) as usize;
                }
            }
            0xFEA0..=0xFEFF => {}
            0xFF7F => {}
            0xFFFF => self.inte = Interrupts::from_bits_retain(v),
            _ => {}
        }
    }
}
