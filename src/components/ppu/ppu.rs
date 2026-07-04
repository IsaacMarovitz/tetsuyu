use crate::components::ppu::bgpi::BGPI;
use crate::components::ppu::cc::ColorCorrection;
use crate::components::ppu::structs::*;
use crate::components::prelude::*;
use crate::config::{Color, Config, PPUConfig, Palette};
use crate::framebuffer::FramebufferWriter;

/// RGBA (4 bytes) per pixel
pub const FRAMEBUFFER_SIZE: usize = 4 * SCREEN_W * SCREEN_H;
pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

pub struct PPU {
    mode: GBMode,
    rom_is_cgb: bool,
    boot_rom_enabled: bool,
    ppu_config: PPUConfig,
    cc: ColorCorrection,
    ppu_mode: PPUMode,
    mode3_len: u32,
    draw_start: u32,
    draw_x: u8,
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
    obp0: u8,
    obp1: u8,
    lcdc: LCDC,
    lcds: LCDS,
    bcps: BGPI,
    bcpd: [u8; 64],
    ocps: BGPI,
    ocpd: [u8; 64],
    vram: [u8; 0x4000],
    vram_bank: usize,
    oam: [u8; 0xA0],
    opri: bool,
    bgprio: [Priority; SCREEN_W],
    pub interrupts: Interrupts,
    framebuffer: FramebufferWriter,
    pub entered_hblank: bool,
    stat_line: bool,
}

impl PPU {
    pub fn new(config: Config, framebuffer: FramebufferWriter, rom_is_cgb: bool) -> Self {
        Self {
            mode: config.mode,
            rom_is_cgb,
            boot_rom_enabled: true,
            ppu_config: config.ppu_config,
            cc: ColorCorrection::new(),
            ppu_mode: PPUMode::OAMScan,
            mode3_len: 0,
            draw_start: 0,
            draw_x: 0,
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
            obp0: 0x00,
            obp1: 0x01,
            lcdc: LCDC::empty(),
            lcds: LCDS::empty(),
            bcps: BGPI::new(),
            bcpd: [0; 64],
            ocps: BGPI::new(),
            ocpd: [0; 64],
            vram: [0; 0x4000],
            vram_bank: 0,
            oam: [0; 0xA0],
            opri: true,
            bgprio: [Priority::Normal; SCREEN_W],
            interrupts: Interrupts::empty(),
            framebuffer,
            entered_hblank: false,
            stat_line: false,
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        if !self.lcdc.contains(LCDC::LCD_ENABLE) {
            return;
        }

        self.cycle_count += cycles;

        match self.ppu_mode {
            PPUMode::OAMScan => {
                self.check_lyc();

                if self.cycle_count >= 80 {
                    self.ppu_mode = PPUMode::Draw;
                    self.mode3_len = 172 + (self.scx % 8) as u32;
                    if self.window_visible() {
                        self.mode3_len += 6;
                    }
                    self.mode3_len += self.mode3_sprite_penalty();
                    self.draw_start = 92 + (self.scx % 8) as u32; // 80 (mode 2) + 12 warmup + fine scroll
                    self.draw_x = 0;
                    self.update_stat_line();
                }
            }
            PPUMode::Draw => {
                let target_x = self
                    .cycle_count
                    .saturating_sub(self.draw_start)
                    .min(SCREEN_W as u32) as u8;

                while self.draw_x < target_x {
                    self.draw_bg_pixel(self.draw_x as usize);
                    self.draw_x += 1;
                }

                if self.cycle_count >= 80 + self.mode3_len {
                    self.ppu_mode = PPUMode::HBlank;
                    self.entered_hblank = true;
                    self.update_stat_line();

                    while self.draw_x < SCREEN_W as u8 {
                        self.draw_bg_pixel(self.draw_x as usize);
                        self.draw_x += 1;
                    }

                    if self.lcdc.contains(LCDC::OBJ_ENABLE) {
                        self.draw_sprites();
                    }
                }
            }
            PPUMode::HBlank => {
                if self.cycle_count >= 456 {
                    self.cycle_count -= 456;
                    self.inc_ly();
                    self.check_lyc();

                    if self.ly > 143 {
                        self.ppu_mode = PPUMode::VBlank;
                        self.interrupts |= Interrupts::V_BLANK;
                        self.update_stat_line();
                        self.framebuffer.submit_frame();
                    } else {
                        self.ppu_mode = PPUMode::OAMScan;
                        self.update_stat_line();
                    }
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
                        self.update_stat_line();
                    } else {
                        self.inc_ly()
                    }

                    self.check_lyc();
                }
            }
        }
    }

    fn check_lyc(&mut self) {
        if self.ly == self.lc {
            self.lcds |= LCDS::LYC_EQUALS;
        } else {
            self.lcds &= !LCDS::LYC_EQUALS;
        }

        self.update_stat_line();
    }

    fn update_stat_line(&mut self) {
        // All enabled STAT sources feed one internal line; the LCD interrupt
        // fires only on its rising edge (STAT blocking).
        let line = (self.lcds.contains(LCDS::LYC_SELECT)
            && self.lcds.contains(LCDS::LYC_EQUALS))
            || (self.lcds.contains(LCDS::MODE_0_SELECT) && self.ppu_mode == PPUMode::HBlank)
            || (self.lcds.contains(LCDS::MODE_1_SELECT) && self.ppu_mode == PPUMode::VBlank)
            || (self.lcds.contains(LCDS::MODE_2_SELECT) && self.ppu_mode == PPUMode::OAMScan);

        if line && !self.stat_line {
            self.interrupts |= Interrupts::LCD;
        }
        self.stat_line = line;
    }

    fn mode3_sprite_penalty(&self) -> u32 {
        // DMG skips object fetches entirely when objects are disabled; CGB
        // still fetches them during Mode 3 (only pixel mixing checks LCDC),
        // so disabling objects shortens Mode 3 on DMG only.
        if self.mode == GBMode::DMG && !self.lcdc.contains(LCDC::OBJ_ENABLE) {
            return 0;
        }

        let sprite_size: i32 = if self.lcdc.contains(LCDC::OBJ_SIZE) { 16 } else { 8 };
        let mut penalty = 0;
        let mut count = 0;

        for i in 0..40 {
            let oam_i = i * 4;
            let y = self.oam[oam_i] as i32;
            let ly = self.ly as i32 + 16;
            if ly >= y && ly < y + sprite_size {
                let x = self.oam[oam_i + 1] as u32;
                penalty += 11 - ((x + self.scx as u32) % 8).min(5);
                count += 1;
                if count == 10 {
                    break;
                }
            }
        }

        penalty
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
            0x00 => palette.light,
            0x01 => palette.light_gray,
            0x02 => palette.dark_gray,
            _ => palette.dark,
        }
    }

    fn set_rgb_mapped(&mut self, x: usize, color: u16) {
        let color = match self.ppu_config.cc_mode {
            CCMode::True => self.cc.true_color_lut[color as usize],
            CCMode::CGB => self.cc.cgb_color_lut[color as usize],
            CCMode::GBA => self.cc.gba_color_lut[color as usize],
            CCMode::SGB => self.cc.sgb_color_lut[color as usize],
        };

        self.framebuffer
            .set_pixel(color[0], color[1], color[2], x, self.ly as usize);
    }

    fn draw_bg_pixel(&mut self, x: usize) {
        // ┌──────────────────┬──────────────────┬──────────────────┐
        // │ TILE_DATA_AREA   │        1         │        0         │
        // ╞══════════════════╪══════════════════╪══════════════════╡
        // │ Range 0-127      │ $8000 - $87FF    │ $8800 - $8FFF    │
        // ├──────────────────┼──────────────────┼──────────────────┤
        // │ Range 128-255    │ $8800 - $8FFF    │ $9000 - $97FF    │
        // └──────────────────┴──────────────────┴──────────────────┘
        let tile_data_base = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
            0x8000
        } else {
            0x8800
        };

        // WX (Window Space) -> WX (Screen Space)
        let wx = self.wx.wrapping_sub(7);

        // Only show window if it's enabled and it intersects current scanline
        let in_window_y = self.window_visible();

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

        // Check if BG/Window is disabled (DMG game with LCDC bit 0 off)
        let bg_disabled = !self.use_cgb_mode() && !self.lcdc.contains(LCDC::WINDOW_PRIORITY);

        if bg_disabled {
            // BG is disabled: draw color 0 from BGP and set priority to Color0
            self.bgprio[x] = Priority::Color0;

            if self.mode == GBMode::CGB {
                // In CGB mode, use white when BG is disabled
                self.set_rgb_mapped(x, 0x7FFF);
            } else {
                let color = Self::grey_to_l(self.ppu_config.palette, self.bgp, 0);
                self.framebuffer.set_pixel(
                    color.r(),
                    color.g(),
                    color.b(),
                    x,
                    self.ly as usize,
                );
            }
            return; // Skip all the tile reading logic
        }

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
        let tile_index = self.read_vram(tile_address, 0);

        // If we're using the secondary address mode,
        // we need to interpret this tile index as signed
        let tile_offset = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
            tile_index as i16
        } else {
            (tile_index as i8) as i16 + 128
        } as u16
            * 16;

        let tile_data_location = tile_data_base + tile_offset;
        let tile_attributes = if self.mode == GBMode::CGB {
            Attributes::from_bits_retain(self.read_vram(tile_address, 1))
        } else {
            // BG Tiles don't get attributes on DMG
            Attributes::empty()
        };

        let tile_y = if tile_attributes.contains(Attributes::Y_FLIP) {
            7 - py % 8
        } else {
            py % 8
        };
        let tile_x = if tile_attributes.contains(Attributes::X_FLIP) {
            7 - px % 8
        } else {
            px % 8
        };
        let tile_data_address = tile_data_location + ((tile_y * 2) as u16);
        let bank = if self.mode == GBMode::CGB && tile_attributes.contains(Attributes::BANK) {
            1
        } else {
            0
        };

        let tile_data = {
            let a = self.read_vram(tile_data_address, bank);
            let b = self.read_vram(tile_data_address + 1, bank);
            [a, b]
        };

        let color_l = if tile_data[0] & (0x80 >> tile_x) != 0 {
            1
        } else {
            0
        };
        let color_h = if tile_data[1] & (0x80 >> tile_x) != 0 {
            2
        } else {
            0
        };
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
            let palette_no_1 = if self.use_cgb_mode() {
                (tile_attributes.bits() & 0b0000_0111) as usize
            } else {
                0
            };

            let final_color = if !self.use_cgb_mode() {
                ((self.bgp >> (color * 2)) & 0x03) as usize
            } else {
                color
            };

            let palette_address = palette_no_1 * 8 + final_color * 2;

            let color: u16 = (self.bcpd[palette_address] as u16)
                | ((self.bcpd[palette_address + 1] as u16) << 8) & 0x7FFF;

            self.set_rgb_mapped(x, color);
        } else {
            let color = Self::grey_to_l(self.ppu_config.palette, self.bgp, color);
            self.framebuffer
                .set_pixel(color.r(), color.g(), color.b(), x, self.ly as usize);
        }
    }

    fn draw_sprites(&mut self) {
        let sprite_size = if self.lcdc.contains(LCDC::OBJ_SIZE) {
            16
        } else {
            8
        };
        let mut object_count: u8 = 0;

        // Track which sprite has "claimed" each pixel position (regardless of whether it actually drew)
        let mut sprite_pixels: [(Option<u16>, Option<u8>); SCREEN_W] = [(None, None); SCREEN_W];

        for i in 0..40 {
            let sprite_address = 0xFE00 + (i as u16) * 4;
            let py = self.read(sprite_address).wrapping_sub(16);
            let px = self.read(sprite_address + 1).wrapping_sub(8);
            let tile_number = self.read(sprite_address + 2)
                & if self.lcdc.contains(LCDC::OBJ_SIZE) {
                    0xFE
                } else {
                    0xFF
                };
            let tile_attributes = Attributes::from_bits_retain(self.read(sprite_address + 3));

            if py <= 0xFF - sprite_size + 1 {
                if self.ly < py || self.ly > py + sprite_size - 1 {
                    continue;
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

            let tile_data_address = 0x8000_u16 + tile_number as u16 * 16 + tile_y as u16 * 2;
            let bank = if self.mode == GBMode::CGB && tile_attributes.contains(Attributes::BANK) {
                1
            } else {
                0
            };

            let tile_data = {
                let a = self.read_vram(tile_data_address, bank);
                let b = self.read_vram(tile_data_address + 1, bank);
                [a, b]
            };

            object_count += 1;
            if object_count > 10 {
                continue;
            }

            for x in 0..8 {
                let screen_x = px.wrapping_add(x);
                if screen_x >= (SCREEN_W as u8) {
                    continue;
                }

                let tile_x = if tile_attributes.contains(Attributes::X_FLIP) {
                    7 - x
                } else {
                    x
                };

                let color_low = if tile_data[0] & (0x80 >> tile_x) != 0 {
                    1
                } else {
                    0
                };
                let color_high = if tile_data[1] & (0x80 >> tile_x) != 0 {
                    2
                } else {
                    0
                };
                let color = color_high | color_low;
                if color == 0 {
                    continue;
                }

                // Check sprite-to-sprite priority
                let screen_x_usize = screen_x as usize;
                if let (Some(prev_addr), Some(prev_px)) = sprite_pixels[screen_x_usize] {
                    let skip = if self.use_cgb_mode() {
                        // CGB ROM: respect OPRI register
                        if self.opri {
                            // OAM priority
                            prev_addr < sprite_address
                        } else {
                            // X-coordinate priority
                            prev_px < px || (prev_px == px && prev_addr < sprite_address)
                        }
                    } else {
                        // DMG ROM: always use X-coordinate priority
                        prev_px < px || (prev_px == px && prev_addr < sprite_address)
                    };

                    if skip {
                        continue;
                    }
                }

                // Mark this pixel as claimed BEFORE checking BG priority
                // This prevents lower-priority sprites from drawing even if this one is blocked
                sprite_pixels[screen_x_usize] = (Some(sprite_address), Some(px));

                let prio = self.bgprio[screen_x as usize];
                let skip = match self.mode {
                    GBMode::CGB => {
                        if self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                            if prio == Priority::Color0 {
                                false
                            } else if !tile_attributes.contains(Attributes::PRIORITY)
                                && prio != Priority::Priority
                            {
                                false
                            } else {
                                true
                            }
                        } else {
                            tile_attributes.contains(Attributes::PRIORITY)
                                && prio != Priority::Color0
                        }
                    }
                    GBMode::DMG => {
                        tile_attributes.contains(Attributes::PRIORITY) && prio != Priority::Color0
                    }
                };

                if skip {
                    continue;
                }

                if self.mode == GBMode::CGB {
                    let palette_no = if self.use_cgb_mode() {
                        (tile_attributes.bits() & 0b0000_0111) as usize
                    } else {
                        if tile_attributes.contains(Attributes::PALETTE_NO_0) {
                            1
                        } else {
                            0
                        }
                    };

                    let final_color = if !self.use_cgb_mode() {
                        // DMG game: remap color through OBP0/OBP1
                        let obp = if tile_attributes.contains(Attributes::PALETTE_NO_0) {
                            self.obp1
                        } else {
                            self.obp0
                        };
                        ((obp >> (color * 2)) & 0x03) as usize
                    } else {
                        color
                    };

                    let palette_address = palette_no * 8 + final_color * 2;

                    let color: u16 = (self.ocpd[palette_address] as u16)
                        | ((self.ocpd[palette_address + 1] as u16) << 8) & 0x7FFF;

                    self.set_rgb_mapped(screen_x as usize, color);
                } else {
                    let color = if tile_attributes.contains(Attributes::PALETTE_NO_0) {
                        Self::grey_to_l(self.ppu_config.palette, self.obp1, color)
                    } else {
                        Self::grey_to_l(self.ppu_config.palette, self.obp0, color)
                    };

                    self.framebuffer.set_pixel(
                        color.r(),
                        color.g(),
                        color.b(),
                        screen_x as usize,
                        self.ly as usize,
                    );
                }
            }
        }
    }

    pub fn clear_vram(&mut self) {
        self.vram = [0; 0x4000];
    }

    pub fn disable_boot_rom(&mut self) {
        self.boot_rom_enabled = false;
    }

    fn use_cgb_mode(&self) -> bool {
        if self.boot_rom_enabled {
            true
        } else {
            self.rom_is_cgb
        }
    }

    fn read_vram(&self, a: u16, bank: usize) -> u8 {
        self.vram[(bank * 0x2000) + a as usize - 0x8000]
    }

    fn write_vram(&mut self, a: u16, v: u8, bank: usize) {
        self.vram[(bank * 0x2000) + a as usize - 0x8000] = v;
    }

    pub fn write_oam(&mut self, index: u16, v: u8) {
        self.oam[index as usize] = v;
    }
}

impl Memory for PPU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x8000..=0x9FFF => {
                if self.ppu_mode != PPUMode::Draw {
                    self.read_vram(a, self.vram_bank)
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
            0xFF41 => self.lcds.bits() | self.ppu_mode as u8 | 0x80,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lc,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            0xFF4F => if self.mode == GBMode::DMG { 0xFF } else { 0xFE | self.vram_bank as u8 },
            0xFF68 => if self.mode == GBMode::DMG { 0xFF } else { self.bcps.read() | 0x40 },
            0xFF69 => {
                if self.ppu_mode != PPUMode::Draw {
                    self.bcpd[self.bcps.address as usize]
                } else {
                    0xFF
                }
            }
            0xFF6A => if self.mode == GBMode::DMG { 0xFF } else { self.ocps.read() | 0x40 },
            0xFF6B => {
                if self.ppu_mode != PPUMode::Draw {
                    self.ocpd[self.ocps.address as usize]
                } else {
                    0xFF
                }
            }
            0xFF6C => 0xFE | self.opri as u8,
            _ => 0xFF
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x8000..=0x9FFF => {
                if self.ppu_mode != PPUMode::Draw {
                    self.write_vram(a, v, self.vram_bank);
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

                    match self.mode {
                        GBMode::DMG => {
                            let color = self.ppu_config.palette.off;
                            self.framebuffer.fill(color.r(), color.g(), color.b());
                        }
                        GBMode::CGB => {
                            self.framebuffer.clear();
                        }
                    }
                }
            }
            0xFF41 => {
                let sanitised = v & 0b1111_1000 | (self.lcds.bits() & 0b0000_0100);
                self.lcds = LCDS::from_bits_truncate(sanitised);
                if self.mode == GBMode::DMG {
                    // DMG STAT-write bug: the write drops the internal line for
                    // one cycle, so any still-true source re-triggers the edge.
                    self.stat_line = false;
                }
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
            0xFF48 => self.obp0 = v,
            0xFF49 => self.obp1 = v,
            0xFF4A => self.wy = v,
            0xFF4B => self.wx = v,
            0xFF4C => {}
            0xFF4F => self.vram_bank = (v & 0x01) as usize,
            0xFF68 => self.bcps.write(v),
            0xFF69 => {
                if self.ppu_mode != PPUMode::Draw {
                    self.bcpd[self.bcps.address as usize] = v;
                }

                if self.bcps.auto_increment {
                    self.bcps.address += 1;
                    self.bcps.address &= 0x3F;
                }
            }
            0xFF6A => self.ocps.write(v),
            0xFF6B => {
                if self.ppu_mode != PPUMode::Draw {
                    self.ocpd[self.ocps.address as usize] = v;
                }

                if self.ocps.auto_increment {
                    self.ocps.address += 1;
                    self.ocps.address &= 0x3F;
                }
            }
            0xFF6C => self.opri = v != 0,
            _ => {}
        }
    }
}
