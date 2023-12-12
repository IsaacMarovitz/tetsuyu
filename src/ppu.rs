use bitflags::{bitflags, Flags};
use crate::mmu::Interrupts;
use crate::mode::GBMode;

pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

pub struct PPU {
    mode: GBMode,
    ppu_mode: PPUMode,
    cycle_count: u32,
    sy: u8,
    sx: u8,
    ly: u8,
    lc: u8,
    wy: u8,
    wx: u8,
    bgp: u8,
    op0: u8,
    op1: u8,
    lcdc: LCDC,
    lcds: LCDS,
    ram: [u8; 0x4000],
    ram_bank: usize,
    oam: [u8; 0xA0],
    bgprio: [Priority; SCREEN_W],
    pub interrupts: Interrupts,
    pub frame_buffer: Vec<u8>
}

#[derive(PartialEq, Copy, Clone)]
enum Priority {
    Color0,
    Priority,
    Normal
}

#[derive(PartialEq, Copy, Clone)]
enum PPUMode {
    OAMScan = 2,
    Draw = 3,
    HBlank = 0,
    VBlank = 1
}

bitflags! {
    pub struct Attributes: u8 {
        const PRIORITY = 0b1000_0000;
        const Y_FLIP = 0b0100_0000;
        const X_FLIP = 0b0010_0000;
        const PALLETE_NO_0 = 0b0001_0000;
        const BANK = 0b0000_1000;
    }
}

bitflags! {
    pub struct LCDC: u8 {
        // LCD & PPU enable: 0 = Off; 1 = On
        const LCD_ENABLE = 0b1000_0000;
        // Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const WINDOW_AREA = 0b0100_0000;
        // Window enable: 0 = Off; 1 = On
        const WINDOW_ENABLE  = 0b0010_0000;
        // BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
        const TILE_DATA_AREA = 0b0001_0000;
        // BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const TILE_MAP_AREA = 0b0000_1000;
        // OBJ size: 0 = 8×8; 1 = 8×16
        const OBJ_SIZE = 0b0000_01000;
        // OBJ enable: 0 = Off; 1 = On
        const OBJ_ENABLE = 0b0000_0010;
        // BG & Window enable (GB) / priority (CGB): 0 = Off; 1 = On
        const WINDOW_PRIORITY = 0b0000_0001;
    }
}

bitflags! {
    pub struct LCDS: u8 {
        // LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt.
        const LYC_SELECT = 0b0100_0000;
        // Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt.
        const MODE_2_SELECT = 0b0010_0000;
        // Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt.
        const MODE_1_SELECT = 0b0001_0000;
        // Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt.
        const MODE_0_SELECT = 0b0000_1000;
        // LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        const LYC_EQUALS = 0b0000_0100;
        // PPU mode (Read-only): Indicates the PPU’s current status.
    }
}
impl PPU {
    pub fn new(mode: GBMode) -> Self {
        Self {
            mode,
            ppu_mode: PPUMode::OAMScan,
            cycle_count: 0,
            sy: 0x00,
            sx: 0x00,
            ly: 0x00,
            lc: 0x00,
            wy: 0x00,
            wx: 0x00,
            bgp: 0x00,
            op0: 0x00,
            op1: 0x01,
            lcdc: LCDC::empty(),
            lcds: LCDS::empty(),
            ram: [0; 0x4000],
            ram_bank: 0,
            oam: [0; 0xA0],
            bgprio: [Priority::Normal; SCREEN_W],
            interrupts: Interrupts::empty(),
            frame_buffer: vec![0x00; 4 * SCREEN_W * SCREEN_H]
        }
    }

    pub fn cycle(&mut self, cycles: u32) -> bool {
        if !self.lcdc.contains(LCDC::LCD_ENABLE) {
            return false;
        }

        self.cycle_count += cycles;

        if self.ly == self.lc {
            self.lcds |= LCDS::LYC_EQUALS;
            if self.lcds.contains(LCDS::LYC_SELECT) {
                self.interrupts |= Interrupts::LCD;
            }
        }

        return match self.ppu_mode {
            PPUMode::OAMScan => {
                if self.cycle_count > 80 {
                    self.cycle_count -= 80;
                    self.ppu_mode = PPUMode::Draw;
                    // println!("[PPU] Switching to Draw!");
                }
                false
            },
            PPUMode::Draw => {
                // TODO: Allow variable length Mode 3
                if self.cycle_count > 172 {
                    self.ppu_mode = PPUMode::HBlank;
                    if self.lcds.contains(LCDS::MODE_0_SELECT) {
                        self.interrupts |= Interrupts::LCD;
                    }
                    //if self.mode == GBMode::Color || self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                    if true {
                        self.draw_bg();
                    }
                    // if self.lcdc.contains(LCDC::OBJ_ENABLE) {
                    if true {
                        self.draw_sprites();
                    }
                    // println!("[PPU] Switching to HBlank!");
                    true
                } else {
                    false
                }
            },
            PPUMode::HBlank => {
                if self.cycle_count > 456 {
                    self.ly += 1;
                    self.cycle_count -= 456;

                    if self.ly > 143 {
                        self.ppu_mode = PPUMode::VBlank;
                        self.interrupts |= Interrupts::V_BLANK;
                        if self.lcds.contains(LCDS::MODE_1_SELECT) {
                            self.interrupts |= Interrupts::LCD;
                        }
                        // println!("[PPU] Switching to VBlank!");
                    } else {
                        self.ppu_mode = PPUMode::OAMScan;
                        if self.lcds.contains(LCDS::MODE_2_SELECT) {
                            self.interrupts |= Interrupts::LCD;
                        }
                        // println!("[PPU] Switching to OAMScan!");
                    }
                }
                false
            },
            PPUMode::VBlank => {
                if self.cycle_count > 4560 {
                    self.cycle_count -= 4560;
                    self.ly = 0;
                    self.ppu_mode = PPUMode::OAMScan;
                    if self.lcds.contains(LCDS::MODE_2_SELECT) {
                        self.interrupts |= Interrupts::LCD;
                    }
                    // println!("[PPU] Switching to OAMScan!");
                }
                false
            }
        }
    }

    fn grey_to_l(v: u8, i: usize) -> u8 {
        match v >> (2 * i) & 0x03 {
            0x00 => 0xFF,
            0x01 => 0xFC,
            0x02 => 0x60,
            _ => 0x00
        }
    }

    fn set_rgb(&mut self, x: usize, r: u8, g: u8, b: u8) {
        // TODO: Color mapping from CGB -> sRGB
        let bytes_per_pixel = 4;
        let bytes_per_row = bytes_per_pixel * SCREEN_W;
        let vertical_offset = self.ly as usize * bytes_per_row;
        let horizontal_offset = x * bytes_per_pixel;
        let total_offset = vertical_offset + horizontal_offset;

        self.frame_buffer[total_offset + 0] = r;
        self.frame_buffer[total_offset + 1] = g;
        self.frame_buffer[total_offset + 2] = b;
        self.frame_buffer[total_offset + 3] = 0xFF;
    }

    fn draw_bg(&mut self) {
        let show_window = self.lcdc.contains(LCDC::WINDOW_ENABLE) && self.wy <= self.ly;
        let tile_base = if self.lcdc.contains(LCDC::TILE_DATA_AREA) { 0x8000 } else { 0x8800 };

        let wx = self.wx.wrapping_sub(7);
        let py = if show_window {
            self.ly.wrapping_sub(self.wy)
        } else {
            self.sy.wrapping_sub(self.ly)
        };
        let tile_index_y = (py as u16 >> 3) & 31;

        for x in 0..SCREEN_W {
            let px = if show_window && x as u8 >= wx {
                x as u8 - wx
            } else {
                self.sx.wrapping_add(x as u8)
            };
            let tile_index_x = (px as u16 >> 3) & 31;

            let bg_base = if show_window && x as u8 >= wx {
                if self.lcdc.contains(LCDC::WINDOW_AREA) {
                    0x9C00
                } else {
                    0x9800
                }
            } else if self.lcdc.contains(LCDC::TILE_MAP_AREA) {
                0x9C00
            } else {
                0x9800
            };

            let tile_address = bg_base + tile_index_y * 32 + tile_index_x;
            let tile_number = self.read_ram0(tile_address);
            let tile_offset = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
                tile_number as i16
            } else {
                tile_number as i8 as i16 + 128
            } as u16 * 16;
            let tile_location = tile_base + tile_offset;
            let tile_attributes = Attributes::from_bits(self.read_ram1(tile_address)).unwrap();

            let tile_y = if tile_attributes.contains(Attributes::Y_FLIP) { 7 - py % 8 } else { py % 8 };
            let tile_y_data = if self.mode == GBMode::Color && tile_attributes.contains(Attributes::BANK) {
                let a = self.read_ram1(tile_location + (tile_y * 2) as u16);
                let b = self.read_ram1(tile_location + (tile_y * 2) as u16 + 1);
                [a, b]
            } else {
                let a = self.read_ram0(tile_location + (tile_y * 2) as u16);
                let b = self.read_ram0(tile_location + (tile_y * 2) as u16 + 1);
                [a, b]
            };

            let tile_x = if tile_attributes.contains(Attributes::X_FLIP) { 7 - px % 8 } else { px % 8 };

            let color_low = if tile_y_data[0] & (0x80 >> tile_x) != 0 { 1 } else { 0 };
            let color_high = if tile_y_data[1] & (0x80 >> tile_x) != 0 { 2 } else { 0 };
            let color = color_high | color_low;

            self.bgprio[x] = if color == 0 {
                Priority::Color0
            } else {
                if tile_attributes.contains(Attributes::PRIORITY) {
                    Priority::Priority
                } else {
                    Priority::Normal
                }
            };

            if self.mode == GBMode::Color {
                let r = 0;
                let g = 0;
                let b = 0;
                self.set_rgb(x, r, g, b);
            } else {
                let lightness = Self::grey_to_l(self.bgp, color);
                self.set_rgb(x, lightness, lightness, lightness);
            }
        }
    }

    fn draw_sprites(&mut self) {
        let sprite_size = if self.lcdc.contains(LCDC::OBJ_SIZE) { 16 } else { 8 };

        for i in 0..40 {
            let sprite_address = 0xFE00 + (i as u16) * 4;
            let py = self.read(sprite_address).wrapping_add(16);
            let px = self.read(sprite_address + 1).wrapping_add(8);
            let tile_number = self.read(sprite_address + 2) & if self.lcdc.contains(LCDC::OBJ_SIZE) { 0xFE } else { 0xFF };
            let tile_attributes = Attributes::from_bits(self.read(sprite_address + 3)).unwrap();

            if py <= 0xFF - sprite_size + 1 {
                if self.ly < py || self.ly > py + sprite_size - 1 {
                    continue
                }
            } else {
                if self.ly > py.wrapping_add(sprite_size) - 1 {
                    continue;
                }
            }

            if px >= (SCREEN_W as u8) && px <= (0xFF - 7) {
                continue;
            }

            let tile_y = if tile_attributes.contains(Attributes::Y_FLIP) {
                sprite_size - 1 - self.ly.wrapping_sub(py)
            } else {
                self.ly.wrapping_sub(py)
            };
            let tile_y_address: u16 = 0x8000_u16 + tile_number as u16 * 16 + tile_y as u16 * 2;
            let tile_y_data = if self.mode == GBMode::Color && tile_attributes.contains(Attributes::BANK) {
                let b1 = self.read_ram1(tile_y_address);
                let b2 = self.read_ram1(tile_y_address + 1);
                [b1, b2]
            } else {
                let b1 = self.read_ram0(tile_y_address);
                let b2 = self.read_ram0(tile_y_address + 1);
                [b1, b2]
            };

            for x in 0..8 {
                if px.wrapping_add(x) >= (SCREEN_W as u8) {
                    continue;
                }
                let tile_x = if tile_attributes.contains(Attributes::X_FLIP) { 7 - x } else { x };

                let color_low = if tile_y_data[0] & (0x80 >> tile_x) != 0 { 1 } else { 0 };
                let color_high = if tile_y_data[1] & (0x80 >> tile_x) != 0 { 2 } else { 0 };
                let color = color_high | color_low;
                if color == 0 {
                    continue;
                }

                // let prio = self.bgprio[px.wrapping_add(x) as usize];
                // let skip = if self.mode == GBMode::Color && !self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                //     prio == Priority::Priority
                // } else if prio == Priority::Priority {
                //     prio != Priority::Priority
                // } else {
                //     tile_attributes.contains(Attributes::PRIORITY) && prio != Priority::Color0
                // };
                // if skip {
                //     continue;
                // }

                if self.mode == GBMode::Color {

                } else {
                    let color = if tile_attributes.contains(Attributes::PALLETE_NO_0) {
                        Self::grey_to_l(self.op1, color)
                    } else {
                        Self::grey_to_l(self.op0, color)
                    };

                    self.set_rgb(px.wrapping_add(x) as usize, color, color, color);
                }
            }
        }
    }

    fn read_ram0(&self, a: u16) -> u8 {
        self.ram[a as usize - 0x8000]
    }

    fn read_ram1(&self, a: u16) -> u8 {
        self.ram[a as usize - 0x6000]
    }

    pub fn read(&self, a: u16) -> u8 {
        match a {
            0x8000..=0x9FFF => self.ram[self.ram_bank * 0x2000 + a as usize - 0x8000],
            0xFE00..=0xFE9F => self.oam[a as usize - 0xFE00],
            0xFF40 => self.lcdc.bits(),
            0xFF41 => self.lcds.bits() | self.ppu_mode as u8,
            0xFF42 => self.sy,
            0xFF43 => self.sx,
            0xFF44 => self.ly,
            0xFF45 => self.lc,
            0xFF47 => self.bgp,
            0xFF48 => self.op0,
            0xFF49 => self.op1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            0xFF4F => 0xFE | self.ram_bank as u8,
            _ => panic!("Read to unsupported PPU address ({:#06x})!", a),
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0x8000..=0x9FFF => self.ram[self.ram_bank * 0x2000 + a as usize - 0x8000] = v,
            0xFE00..=0xFE9F => self.oam[a as usize - 0xFE00] = v,
            0xFF40 => {
                self.lcdc = LCDC::from_bits(v).unwrap();
                if !self.lcdc.contains(LCDC::LCD_ENABLE) {
                    self.ly = 0;
                    self.ppu_mode = PPUMode::HBlank;
                    self.frame_buffer = vec![0x00; 4 * SCREEN_W * SCREEN_H];
                }
            },
            // TODO: Don't allow read-only bits to be set!
            0xFF41 => self.lcds = LCDS::from_bits(v).unwrap(),
            0xFF42 => self.sy = v,
            0xFF43 => self.sx = v,
            0xFF44 => print!("Attempted to write to LY!"),
            0xFF45 => self.lc = v,
            0xFF47 => self.bgp = v,
            0xFF48 => self.op0 = v,
            0xFF49 => self.op1 = v,
            0xFF4A => self.wy = v,
            0xFF4B => self.wx = v,
            0xFF4F => self.ram_bank = (v & 0x01) as usize,
            _ => panic!("Write to unsupported PPU address ({:#06x})!", a),
        }
    }
}