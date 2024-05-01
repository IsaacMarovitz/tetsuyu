use crate::components::prelude::*;
use crate::config::{Color, Config, Palette};
use crate::Framebuffer;
use bitflags::bitflags;

/// RGBA (4 bytes) per pixel
pub const FRAMEBUFFER_SIZE: usize = 4 * SCREEN_W * SCREEN_H;
pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

pub struct PPU {
    mode: GBMode,
    palette: Palette,
    ppu_mode: PPUMode,
    cycle_count: u32,
    vblanked_lines: u32,
    scy: u8,
    scx: u8,
    ly: u8,
    lc: u8,
    wy: u8,
    wx: u8,
    wly: u8,
    bgp: u8,
    op0: u8,
    op1: u8,
    lcdc: LCDC,
    lcds: LCDS,
    bcps: BGPI,
    bcpd: [[[u8; 3]; 4]; 8],
    ocps: BGPI,
    ocpd: [[[u8; 3]; 4]; 8],
    vram: [u8; 0x4000],
    vram_bank: usize,
    oam: [u8; 0xA0],
    opri: bool,
    bgprio: [Priority; SCREEN_W],
    pub interrupts: Interrupts,
    pub framebuffer: Framebuffer,
}

#[derive(PartialEq, Copy, Clone)]
enum Priority {
    Color0,
    Priority,
    Normal,
}

#[derive(PartialEq, Copy, Clone)]
enum PPUMode {
    OAMScan = 2,
    Draw = 3,
    HBlank = 0,
    VBlank = 1,
}

bitflags! {
    #[derive(PartialEq, Copy, Clone)]
    pub struct Attributes: u8 {
        const PRIORITY     = 0b1000_0000;
        const Y_FLIP       = 0b0100_0000;
        const X_FLIP       = 0b0010_0000;
        const PALETTE_NO_0 = 0b0001_0000;
        const BANK         = 0b0000_1000;
    }
}

bitflags! {
    #[derive(PartialEq, Copy, Clone)]
    pub struct LCDC: u8 {
        // LCD & PPU enable: 0 = Off; 1 = On
        const LCD_ENABLE      = 0b1000_0000;
        // Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const WINDOW_AREA     = 0b0100_0000;
        // Window enable: 0 = Off; 1 = On
        const WINDOW_ENABLE   = 0b0010_0000;
        // BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
        const TILE_DATA_AREA  = 0b0001_0000;
        // BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const BG_TILE_MAP_AREA   = 0b0000_1000;
        // OBJ size: 0 = 8×8; 1 = 8×16
        const OBJ_SIZE        = 0b0000_0100;
        // OBJ enable: 0 = Off; 1 = On
        const OBJ_ENABLE      = 0b0000_0010;
        // BG & Window enable (GB) / priority (CGB): 0 = Off; 1 = On
        const WINDOW_PRIORITY = 0b0000_0001;
    }
}

bitflags! {
    #[derive(PartialEq, Copy, Clone)]
    pub struct LCDS: u8 {
        // LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt.
        const LYC_SELECT    = 0b0100_0000;
        // Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt.
        const MODE_2_SELECT = 0b0010_0000;
        // Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt.
        const MODE_1_SELECT = 0b0001_0000;
        // Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt.
        const MODE_0_SELECT = 0b0000_1000;
        // LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        const LYC_EQUALS    = 0b0000_0100;
        // PPU mode (Read-only): Indicates the PPU’s current status.
    }
}

struct BGPI {
    i: u8,
    auto_increment: bool
}

impl BGPI {
    fn new() -> Self {
        Self {
            i: 0,
            auto_increment: false
        }
    }

    fn read(&self) -> u8 {
        let a = if self.auto_increment {
            0x80
        } else {
            0x00
        };
        a | self.i
    }

    fn write(&mut self, v: u8) {
        self.auto_increment = v & 0x80 != 0x00;
        self.i = v & 0x3F;
    }
}

impl PPU {
    pub fn new(config: Config, framebuffer: Framebuffer) -> Self {
        Self {
            mode: config.mode,
            palette: config.palette,
            ppu_mode: PPUMode::OAMScan,
            cycle_count: 0,
            vblanked_lines: 0,
            scy: 0x00,
            scx: 0x00,
            ly: 0x00,
            lc: 0x00,
            wy: 0x00,
            wx: 0x00,
            wly: 0x00,
            bgp: 0x00,
            op0: 0x00,
            op1: 0x01,
            lcdc: LCDC::empty(),
            lcds: LCDS::empty(),
            bcps: BGPI::new(),
            bcpd: [[[0; 3]; 4]; 8],
            ocps: BGPI::new(),
            ocpd: [[[0; 3]; 4]; 8],
            vram: [0; 0x4000],
            vram_bank: 0,
            oam: [0; 0xA0],
            opri: true,
            bgprio: [Priority::Normal; SCREEN_W],
            interrupts: Interrupts::empty(),
            framebuffer,
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        if !self.lcdc.contains(LCDC::LCD_ENABLE) {
            return
        }

        self.cycle_count += cycles;

        return match self.ppu_mode {
            PPUMode::OAMScan => {
                self.check_lyc();

                if self.cycle_count >= 80 {
                    self.ppu_mode = PPUMode::Draw;
                    // println!("[PPU] Switching to Draw!");
                }
            }
            PPUMode::Draw => {
                // TODO: Allow variable length Mode 3
                if self.cycle_count >= (172 + 80) {
                    self.ppu_mode = PPUMode::HBlank;
                    if self.lcds.contains(LCDS::MODE_0_SELECT) {
                        self.interrupts |= Interrupts::LCD;
                    }

                    self.draw_bg();

                    if self.lcdc.contains(LCDC::OBJ_ENABLE) {
                        self.draw_sprites();
                    }
                    // println!("[PPU] Switching to HBlank!");
                }
            }
            PPUMode::HBlank => {
                if self.cycle_count >= 456 {
                    self.cycle_count -= 456;
                    self.inc_ly();
                    self.check_lyc();

                    return if self.ly > 143 {
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
                    };
                }
            }
            PPUMode::VBlank => {
                if self.cycle_count >= 456 {
                    self.cycle_count -= 456;
                    self.vblanked_lines += 1;

                    if self.vblanked_lines >= 10 {
                        self.vblanked_lines = 0;
                        self.reset_ly();
                        self.ppu_mode = PPUMode::OAMScan;
                        if self.lcds.contains(LCDS::MODE_2_SELECT) {
                            self.interrupts |= Interrupts::LCD;
                        }
                        // println!("[PPU] Switching to OAMScan!");
                    } else {
                        self.inc_ly()
                    }

                    self.check_lyc();
                }
            }
        };
    }

    fn check_lyc(&mut self) {
        if self.ly == self.lc {
            if self.lcds.contains(LCDS::LYC_SELECT) && !self.lcds.contains(LCDS::LYC_EQUALS) {
                self.interrupts |= Interrupts::LCD;
            }

            self.lcds |= LCDS::LYC_EQUALS;
        } else {
            self.lcds &= !LCDS::LYC_EQUALS;
        }
    }

    fn window_visible(&mut self) -> bool {
        self.wy <= self.ly && self.lcdc.contains(LCDC::WINDOW_ENABLE) && self.wx < 168
    }

    fn inc_ly(&mut self) {
        if self.ppu_mode == PPUMode::VBlank {
            // Always INC in VBlank
            self.wly += 1;
        } else if self.window_visible() {
            // Otherwise, we need to check if the window is visible on this scanline
            self.wly += 1;
        }

        self.ly += 1;
    }

    fn reset_ly(&mut self) {
        self.ly = 0;
        self.wly = 0;
    }

    fn grey_to_l(palette: Palette, v: u8, i: usize) -> Color {
        match v >> (2 * i) & 0x03 {
            0x00 => palette.dark,
            0x01 => palette.dark_gray,
            0x02 => palette.light_gray,
            _ => palette.light,
        }
    }

    fn set_rgb_mapped(&mut self, x: usize, r: u8, g: u8, b: u8) {
        let r_init = r as u32;
        let g_init = g as u32;
        let b_init = b as u32;

        let r = ((r_init * 13 + g_init * 2 + b_init) >> 1) as u8;
        let g = ((g_init * 3 + b_init) << 1) as u8;
        let b = ((r_init * 3 + g_init * 2 + b_init * 11) >> 1) as u8;

        self.set_rgb(x, r, g, b);
    }

    fn set_rgb(&mut self, x: usize, r: u8, g: u8, b: u8) {
        let bytes_per_pixel = 4;
        let bytes_per_row = bytes_per_pixel * SCREEN_W;
        let vertical_offset = self.ly as usize * bytes_per_row;
        let horizontal_offset = x * bytes_per_pixel;
        let total_offset = vertical_offset + horizontal_offset;

        let mut framebuffer = self.framebuffer.write().unwrap();
        framebuffer[total_offset + 0] = r;
        framebuffer[total_offset + 1] = g;
        framebuffer[total_offset + 2] = b;
        framebuffer[total_offset + 3] = 0xFF;
    }

    fn draw_bg(&mut self) {
        // If TILE_DATA_AREA = 1  TILE_DATA_AREA = 0
        // 0-127   = $8000-$87FF;        $8800-$8FFF
        // 128-255 = $8800-$8FFF;        $9000-$97FF
        let tile_data_base = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
            0x8000
        } else {
            0x8800
        };

        // WX (Window Space) -> WX (Screen Space)
        let wx = self.wx.wrapping_sub(7);

        // Only show window if it's enabled and it intersects current scanline
        let in_window_y = self.window_visible();

        for x in 0..SCREEN_W {
            let in_window_x = x as u8 >= wx;

            let (px, py) = if in_window_y && in_window_x {
                let px = x as u8 - wx;
                let py = self.wly;
                (px, py)
            } else {
                let px = self.scx.wrapping_add(x as u8);
                let py = self.scy.wrapping_add(self.ly);
                (px, py)
            };

            // Tile Map Base Address
            let tile_map_base = if in_window_y && in_window_x {
                if self.lcdc.contains(LCDC::WINDOW_AREA) {
                    0x9C00
                } else {
                    0x9800
                }
            } else if self.lcdc.contains(LCDC::BG_TILE_MAP_AREA) {
                0x9C00
            } else {
                0x9800
            };

            let tile_index_y = (py as u16 >> 3) & 31;
            let tile_index_x = (px as u16 >> 3) & 31;

            // Location of Tile Attributes
            let tile_address = tile_map_base + tile_index_y * 32 + tile_index_x;
            let tile_index = self.read_ram0(tile_address);

            // If we're using the secondary address mode,
            // we need to interpret this tile index as signed
            let tile_offset = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
                tile_index as i16
            } else {
                (tile_index as i8) as i16 + 128
            } as u16 * 16;

            let tile_data_location = tile_data_base + tile_offset;
            let tile_attributes = Attributes::from_bits_retain(self.read_ram1(tile_address));

            let tile_y = if tile_attributes.contains(Attributes::Y_FLIP) { 7 - py % 8 } else { py % 8 };
            let tile_x = if tile_attributes.contains(Attributes::X_FLIP) { 7 - px % 8 } else { px % 8 };

            let tile_y_data = if self.mode == GBMode::CGB && tile_attributes.contains(Attributes::BANK) {
                let a = self.read_ram1(tile_data_location + ((tile_y * 2) as u16));
                let b = self.read_ram1(tile_data_location + ((tile_y * 2) as u16) + 1);
                [a, b]
            } else {
                let a = self.read_ram0(tile_data_location + ((tile_y * 2) as u16));
                let b = self.read_ram0(tile_data_location + ((tile_y * 2) as u16) + 1);
                [a, b]
            };

            let color_l = if tile_y_data[0] & (0x80 >> tile_x) != 0 { 1 } else { 0 };
            let color_h = if tile_y_data[1] & (0x80 >> tile_x) != 0 { 2 } else { 0 };
            let color = color_h | color_l;

            self.bgprio[x] = if color == 0 {
                Priority::Color0
            } else {
                if tile_attributes.contains(Attributes::PRIORITY) {
                    Priority::Priority
                } else {
                    Priority::Normal
                }
            };

            if self.mode == GBMode::CGB {
                let palette_no_1 = (tile_attributes.bits() & 0b0000_0111) as usize;
                let r = self.bcpd[palette_no_1][color][0];
                let g = self.bcpd[palette_no_1][color][1];
                let b = self.bcpd[palette_no_1][color][2];
                self.set_rgb_mapped(x, r, g, b);
            } else {
                let color = if !self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                    Self::grey_to_l(self.palette.clone(), self.bgp, 0)
                } else {
                    Self::grey_to_l(self.palette.clone(), self.bgp, color)
                };

                self.set_rgb(x, color.r, color.g, color.b);
            }
        }
    }

    fn draw_sprites(&mut self) {
        let sprite_size = if self.lcdc.contains(LCDC::OBJ_SIZE) { 16 } else { 8 };
        let mut object_count: u8 = 0;
        let mut previous_px: u8 = 0;
        // Start this with max value, otherwise first
        // sprite will always be skipped
        let mut previous_address: u16 = u16::MAX;

        for i in 0..40 {
            let sprite_address = 0xFE00 + (i as u16) * 4;
            let py = self.read(sprite_address).wrapping_sub(16);
            let px = self.read(sprite_address + 1).wrapping_sub(8);
            let tile_number = self.read(sprite_address + 2) & if self.lcdc.contains(LCDC::OBJ_SIZE) { 0xFE } else { 0xFF };
            let tile_attributes = Attributes::from_bits_truncate(self.read(sprite_address + 3));

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

            // TODO: Respect OPRI
            if previous_px == px {
                if previous_address < sprite_address {
                    continue;
                }
            }

            previous_px = px;
            previous_address = sprite_address;

            let tile_y = if tile_attributes.contains(Attributes::Y_FLIP) {
                sprite_size - 1 - self.ly.wrapping_sub(py)
            } else {
                self.ly.wrapping_sub(py)
            };
            let tile_y_address: u16 = 0x8000_u16 + tile_number as u16 * 16 + tile_y as u16 * 2;
            let tile_y_data = if self.mode == GBMode::CGB && tile_attributes.contains(Attributes::BANK) {
                let b1 = self.read_ram1(tile_y_address);
                let b2 = self.read_ram1(tile_y_address + 1);
                [b1, b2]
            } else {
                let b1 = self.read_ram0(tile_y_address);
                let b2 = self.read_ram0(tile_y_address + 1);
                [b1, b2]
            };

            object_count += 1;
            if object_count > 10 {
                continue;
            }

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

                let prio = self.bgprio[px.wrapping_add(x) as usize];
                let skip = match self.mode {
                    GBMode::CGB => {
                        if self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                            prio == Priority::Priority
                        } else {
                            tile_attributes.contains(Attributes::PRIORITY) && prio != Priority::Color0
                        }
                    },
                    GBMode::DMG => {
                        tile_attributes.contains(Attributes::PRIORITY) && prio != Priority::Color0
                    }
                };

                if skip {
                    continue;
                }

                if self.mode == GBMode::CGB {
                    let palette_no_1 = (tile_attributes.bits() & 0b0000_0111) as usize;
                    let r = self.ocpd[palette_no_1][color][0];
                    let g = self.ocpd[palette_no_1][color][1];
                    let b = self.ocpd[palette_no_1][color][2];
                    self.set_rgb_mapped(px.wrapping_add(x) as usize, r, g, b);
                } else {
                    let color = if tile_attributes.contains(Attributes::PALETTE_NO_0) {
                        Self::grey_to_l(self.palette.clone(), self.op1, color)
                    } else {
                        Self::grey_to_l(self.palette.clone(), self.op0, color)
                    };

                    self.set_rgb(px.wrapping_add(x) as usize, color.r, color.g, color.b);
                }
            }
        }
    }

    fn read_ram0(&self, a: u16) -> u8 {
        self.vram[a as usize - 0x8000]
    }

    fn read_ram1(&self, a: u16) -> u8 {
        self.vram[a as usize - 0x6000]
    }
}

impl Memory for PPU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x8000..=0x9FFF => {
                if self.ppu_mode != PPUMode::Draw {
                    self.vram[(self.vram_bank * 0x2000) + a as usize - 0x8000]
                } else {
                    0xFF
                }
            }
            0xFE00..=0xFE9F => {
                if self.ppu_mode != PPUMode::Draw && self.ppu_mode != PPUMode::OAMScan {
                    self.oam[a as usize - 0xFE00]
                } else {
                    0xFF
                }
            }
            0xFF40 => self.lcdc.bits(),
            0xFF41 => self.lcds.bits() | self.ppu_mode as u8,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lc,
            0xFF47 => self.bgp,
            0xFF48 => self.op0,
            0xFF49 => self.op1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            0xFF4D => 0x00,
            0xFF4F => 0xFE | self.vram_bank as u8,
            // TODO: DMA
            0xFF51..=0xFF55 => 0x00,
            0xFF68 => self.bcps.read(),
            0xFF69 => {
                let r = self.bcps.i as usize >> 3;
                let c = self.bcps.i as usize >> 1 & 0x03;
                if self.bcps.i & 0x01 == 0x00 {
                    let a = self.bcpd[r][c][0] >> 0;
                    let b = self.bcpd[r][c][1] << 5;
                    a | b
                } else {
                    let a = self.bcpd[r][c][1] >> 3;
                    let b = self.bcpd[r][c][2] << 2;
                    a | b
                }
            }
            0xFF6A => self.ocps.read(),
            0xFF6B => {
                let r = self.ocps.i as usize >> 3;
                let c = self.ocps.i as usize >> 1 & 0x03;
                if self.ocps.i & 0x01 == 0x00 {
                    let a = self.ocpd[r][c][0] >> 0;
                    let b = self.ocpd[r][c][1] << 5;
                    a | b
                } else {
                    let a = self.ocpd[r][c][1] >> 3;
                    let b = self.ocpd[r][c][2] << 2;
                    a | b
                }
            }
            0xFF6C => self.opri as u8,
            _ => panic!("Read to unsupported PPU address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x8000..=0x9FFF => {
                if self.ppu_mode != PPUMode::Draw {
                    self.vram[(self.vram_bank * 0x2000) + a as usize - 0x8000] = v
                }
            }
            0xFE00..=0xFE9F => {
                if self.ppu_mode != PPUMode::Draw && self.ppu_mode != PPUMode::OAMScan {
                    self.oam[a as usize - 0xFE00] = v
                }
            }
            0xFF40 => {
                self.lcdc = LCDC::from_bits(v).unwrap();
                if !self.lcdc.contains(LCDC::LCD_ENABLE) {
                    self.reset_ly();
                    self.ppu_mode = PPUMode::HBlank;

                    let mut framebuffer = self.framebuffer.write().unwrap();
                    *framebuffer = [0xFF; FRAMEBUFFER_SIZE];
                }
            }
            0xFF41 => {
                let sanitised = v & 0b1111_1000 | (self.lcds.bits() & 0b0000_0100);
                self.lcds = LCDS::from_bits_truncate(sanitised);
                self.check_lyc();
            }
            0xFF42 => self.scy = v,
            0xFF43 => self.scx = v,
            0xFF44 => println!("Attempted to write to LY!"),
            0xFF45 => {
                self.lc = v;
                self.check_lyc();
            }
            0xFF47 => self.bgp = v,
            0xFF48 => self.op0 = v,
            0xFF49 => self.op1 = v,
            0xFF4A => self.wy = v,
            0xFF4B => self.wx = v,
            0xFF4C => {}
            // TODO: Handle PPU speed switching
            0xFF4D => {}
            0xFF4F => self.vram_bank = (v & 0x01) as usize,
            // TODO: DMA
            0xFF51..=0xFF55 => {}
            0xFF68 => self.bcps.write(v),
            0xFF69 => {
                let r = self.bcps.i as usize >> 3;
                let c = self.bcps.i as usize >> 1 & 0x03;
                if self.bcps.i & 0x01 == 0x00 {
                    self.bcpd[r][c][0] = v & 0x1F;
                    self.bcpd[r][c][1] = (self.bcpd[r][c][1] & 0x18) | (v >> 5);
                } else {
                    self.bcpd[r][c][1] = (self.bcpd[r][c][1] & 0x07) | ((v & 0x03) << 3);
                    self.bcpd[r][c][2] = (v >> 2) & 0x1F;
                }

                if self.bcps.auto_increment {
                    self.bcps.i += 1;
                    self.bcps.i &= 0x3F;
                }
            }
            0xFF6A => self.ocps.write(v),
            0xFF6B => {
                let r = self.ocps.i as usize >> 3;
                let c = self.ocps.i as usize >> 1 & 0x03;
                if self.ocps.i & 0x01 == 0x00 {
                    self.ocpd[r][c][0] = v & 0x1F;
                    self.ocpd[r][c][1] = (self.ocpd[r][c][1] & 0x18) | (v >> 5);
                } else {
                    self.ocpd[r][c][1] = (self.ocpd[r][c][1] & 0x07) | ((v & 0x03) << 3);
                    self.ocpd[r][c][2] = (v >> 2) & 0x1F;
                }

                if self.ocps.auto_increment {
                    self.ocps.i += 1;
                    self.ocps.i &= 0x3F;
                }
            }
            // TODO: Object Priority Mode
            0xFF6C => self.opri = v != 0,
            _ => panic!("Write to unsupported PPU address ({:#06x})!", a),
        }
    }
}
