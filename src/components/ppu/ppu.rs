use crate::components::ppu::bgpi::BGPI;
use crate::components::ppu::cc::ColorCorrection;
use crate::components::ppu::fetcher::*;
use crate::components::ppu::structs::*;
use crate::components::prelude::*;
use crate::config::{Color, Config, PPUConfig, Palette};
use crate::framebuffer::FramebufferWriter;
use crate::hw::interrupt::Interrupts;

/// RGBA (4 bytes) per pixel
pub const FRAMEBUFFER_SIZE: usize = 4 * SCREEN_W * SCREEN_H;
pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

/// Dots before a line formally starts at which its mode-2 (OAM) STAT source
/// is already asserted. On hardware the OAM STAT interrupt for a line is
/// raised at the tail of the previous line's HBlank, giving a mode-2 STAT
/// handler time to run before that line's Mode 3. This is the real hardware
/// edge position, measured against the mealybug m3 mode-2 handlers.
const MODE2_LOOKAHEAD: u32 = 4;

pub struct PPU {
    mode: GBMode,
    rom_is_cgb: bool,
    boot_rom_enabled: bool,
    ppu_config: PPUConfig,
    cc: ColorCorrection,
    ppu_mode: PPUMode,
    cycle_count: u32,
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
    pub interrupts: Interrupts,
    framebuffer: FramebufferWriter,
    pub entered_hblank: bool,
    stat_line: bool,
    bg_fifo: PixelFifo<BgPixel>,
    obj_line: [ObjPixel; SCREEN_W],
    fetcher: Fetcher,
    shifter: Shifter,
    sprites: Vec<SelectedSprite>,
    /// The window is currently driving the fetcher this line (set the dot the
    /// trigger fires, cleared at Mode 3 start).
    window_active: bool,
    /// Latched copy of `window_active` for the whole line: true once the
    /// window activated during this line's Mode 3, so `inc_ly` can advance the
    /// window line counter exactly when hardware does. Distinct from
    /// `window_active`, which is cleared each new Mode 3.
    window_line_active: bool,
    /// The window "Y condition" (Pandocs): cleared each VBlank, and latched
    /// true at the start of any scanline where WY == LY, staying true for the
    /// rest of the frame. The window can only trigger while this holds — it is
    /// sampled once per line, not compared live against LY.
    window_y_condition: bool,
    /// DMG BGP write artifact: `old | new`, visible for exactly one dot.
    bgp_glitch: Option<u8>,
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
            cycle_count: 0,
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
            interrupts: Interrupts::empty(),
            framebuffer,
            entered_hblank: false,
            stat_line: false,
            bg_fifo: PixelFifo::new(),
            obj_line: [ObjPixel::default(); SCREEN_W],
            fetcher: Fetcher::new(),
            shifter: Shifter::new(),
            sprites: Vec::with_capacity(10),
            window_active: false,
            window_line_active: false,
            window_y_condition: false,
            bgp_glitch: None,
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        if !self.lcdc.contains(LCDC::LCD_ENABLE) {
            return;
        }

        for _ in 0..cycles {
            self.dot();
        }
    }

    /// Advance the PPU exactly one dot. `cycle_count` is the dot position
    /// within the current line (0..456); LY is the line. Mode is derived from
    /// the dot position rather than tracked by ad-hoc transition events, so
    /// every mode edge — and the STAT interrupt it drives — lands on the exact
    /// dot hardware produces it.
    fn dot(&mut self) {
        // Mode 3 runs its own per-dot renderer; every other mode is idle
        // between the position-driven edges handled below.
        if self.ppu_mode == PPUMode::Draw {
            self.mode3_dot();
        }

        self.cycle_count += 1;

        match self.ppu_mode {
            PPUMode::OAMScan if self.cycle_count == 80 => {
                // 80 dots of OAM scan complete: begin Mode 3.
                self.ppu_mode = PPUMode::Draw;
                self.start_mode3_fifo();
                self.update_stat_line();
            }
            PPUMode::HBlank if self.cycle_count == 456 - MODE2_LOOKAHEAD && self.ly < 143 => {
                // Hardware asserts the *next* line's mode-2 STAT condition a
                // few dots before the line formally starts (the OAM STAT source
                // is evaluated against the upcoming line at the tail of HBlank).
                // This is a real hardware edge, not a timing fudge: the mode-2
                // interrupt handler must be able to run before the next line's
                // Mode 3.
                self.assert_early_oam_stat();
            }
            _ => {}
        }

        // End of a line.
        if self.cycle_count >= 456 {
            self.cycle_count = 0;
            self.next_line();
        }
    }

    /// Advance to the next line: bump LY, pick the new mode, re-latch LYC and
    /// the STAT line.
    fn next_line(&mut self) {
        self.inc_ly();

        if self.ly > 153 {
            // Wrap back to the top of the frame.
            self.reset_ly();
            self.ppu_mode = PPUMode::OAMScan;
        } else if self.ly == 144 {
            self.ppu_mode = PPUMode::VBlank;
            self.interrupts |= Interrupts::V_BLANK;
            self.framebuffer.submit_frame();
            // Y condition is cleared each VBlank (Pandocs).
            self.window_y_condition = false;
        } else if self.ly < 144 {
            self.ppu_mode = PPUMode::OAMScan;
        }
        // 145..=153 stay in VBlank.

        self.check_lyc();
    }

    /// The mode-2 STAT source, asserted early (during the tail of the previous
    /// line's HBlank) for the upcoming line. Only the mode-2 select bit can
    /// raise the line here; the full re-evaluation happens at `next_line`.
    fn assert_early_oam_stat(&mut self) {
        if self.lcds.contains(LCDS::MODE_2_SELECT) && !self.stat_line {
            self.interrupts |= Interrupts::LCD;
            self.stat_line = true;
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
        let line = (self.lcds.contains(LCDS::LYC_SELECT) && self.lcds.contains(LCDS::LYC_EQUALS))
            || (self.lcds.contains(LCDS::MODE_0_SELECT) && self.ppu_mode == PPUMode::HBlank)
            || (self.lcds.contains(LCDS::MODE_1_SELECT) && self.ppu_mode == PPUMode::VBlank)
            || (self.lcds.contains(LCDS::MODE_2_SELECT) && self.ppu_mode == PPUMode::OAMScan);

        if line && !self.stat_line {
            self.interrupts |= Interrupts::LCD;
        }
        self.stat_line = line;
    }

    /// Set up the FIFO renderer for one Mode 3 (called at the OAMScan→Draw
    /// edge). Selects this line's sprites and resets the fetcher/shifter/FIFO.
    fn start_mode3_fifo(&mut self) {
        self.bg_fifo.clear();
        self.obj_line = [ObjPixel::default(); SCREEN_W];
        self.fetcher = Fetcher::new();
        self.shifter = Shifter::new();
        self.shifter.discard = self.scx % 8;
        self.window_active = false;
        self.window_line_active = false;
        // Latch the Y condition for this scanline: once WY == LY has been seen
        // this frame it stays set (Pandocs "Window rendering criteria").
        if self.wy == self.ly {
            self.window_y_condition = true;
        }
        self.bgp_glitch = None;
        self.select_sprites();
    }

    /// Mode 2 result: up to 10 sprites overlapping this line, in OAM order.
    fn select_sprites(&mut self) {
        self.sprites.clear();
        let size = if self.lcdc.contains(LCDC::OBJ_SIZE) {
            16
        } else {
            8
        };
        let line = self.ly as i32 + 16;
        for i in 0..40u8 {
            let o = i as usize * 4;
            let y = self.oam[o] as i32;
            if line >= y && line < y + size {
                self.sprites.push(SelectedSprite {
                    oam_index: i * 4,
                    x: self.oam[o + 1],
                    y: self.oam[o],
                    tile: self.oam[o + 2],
                    attr: Attributes::from_bits_retain(self.oam[o + 3]),
                    fetched: false,
                });
                if self.sprites.len() == 10 {
                    break;
                }
            }
        }
    }

    /// Advance the renderer by one dot
    fn mode3_dot(&mut self) {
        // A window trigger may clear the BG FIFO before anything shifts.
        self.check_window_trigger();

        // Begin a sprite fetch if one is due (stalls the shifter until done).
        if !self.fetcher.in_sprite_fetch() {
            self.try_start_sprite();
        }

        // One dot of the active fetcher (sprite sub-fetch has priority).
        if self.fetcher.in_sprite_fetch() {
            self.sprite_fetch_step();
        } else {
            self.bg_fetch_step();
        }

        // Shift one pixel unless a sprite fetch is stalling us or the BG FIFO
        // has nothing ready yet.
        if !self.fetcher.in_sprite_fetch() && !self.bg_fifo.is_empty() {
            self.shift_pixel();
        }

        // The window comparator position ticks on every Mode 3 dot the
        // shifter isn't stalled by a sprite fetch.
        if !self.fetcher.in_sprite_fetch() {
            self.shifter.pos = self.shifter.pos.wrapping_add(1);
        }

        // The BGP write artifact lives for exactly the one dot after the write
        // resolves; whether or not a pixel was emitted on it, it's gone now.
        self.bgp_glitch = None;

        if self.shifter.emitted >= SCREEN_W as u8 {
            self.ppu_mode = PPUMode::HBlank;
            self.entered_hblank = true;
            self.update_stat_line();
        }
    }

    /// One dot of the background/window fetch cycle (2 dots per step; Push is a
    /// single dot that only succeeds into an empty FIFO).
    fn bg_fetch_step(&mut self) {
        match self.fetcher.step {
            FetchStep::TileId => {
                if self.fetcher.substep == 1 {
                    self.bg_fetch_tile_id();
                    self.fetcher.step = FetchStep::TileLow;
                }
                self.fetcher.substep ^= 1;
            }
            FetchStep::TileLow => {
                if self.fetcher.substep == 1 {
                    self.fetcher.tile_low = self.bg_fetch_plane(0);
                    self.fetcher.step = FetchStep::TileHigh;
                }
                self.fetcher.substep ^= 1;
            }
            FetchStep::TileHigh => {
                if self.fetcher.substep == 1 {
                    self.fetcher.tile_high = self.bg_fetch_plane(1);
                    if !self.fetcher.done_dummy {
                        // Hardware discards the line's first fetch the moment
                        // it completes and restarts against the same column:
                        // the 12-dot warm-up is two clean 6-dot fetches, with
                        // no extra push dot, putting the first pixel on the
                        // LCD at dot 92.
                        self.fetcher.done_dummy = true;
                        self.fetcher.restart_bg();
                        return;
                    }
                    self.fetcher.step = FetchStep::Push;
                }
                self.fetcher.substep ^= 1;
            }
            FetchStep::Push => {
                if self.bg_fifo.is_empty() {
                    let row = self.assemble_bg_row();
                    self.bg_fifo.push_row(row);
                    self.fetcher.tile_x = self.fetcher.tile_x.wrapping_add(1);
                    self.fetcher.restart_bg();
                }
                // else: FIFO still draining — idle this dot.
            }
        }
    }

    /// Fetch the tile id (and CGB attribute) for the fetcher's current column.
    fn bg_fetch_tile_id(&mut self) {
        let (base, tx, ty) = if self.fetcher.fetching_window {
            let base = if self.lcdc.contains(LCDC::WINDOW_AREA) {
                0x9C00
            } else {
                0x9800
            };
            (
                base,
                self.fetcher.tile_x as u16 & 31,
                (self.wly as u16 >> 3) & 31,
            )
        } else {
            let base = if self.lcdc.contains(LCDC::BG_TILE_MAP_AREA) {
                0x9C00
            } else {
                0x9800
            };
            let tx = ((self.scx as u16 / 8) + self.fetcher.tile_x as u16) & 31;
            let ty = ((self.scy.wrapping_add(self.ly) as u16) >> 3) & 31;
            (base, tx, ty)
        };
        let addr = base + ty * 32 + tx;
        self.fetcher.tile_id = self.read_vram(addr, 0);
        self.fetcher.tile_attr = if self.mode == GBMode::CGB {
            self.read_vram(addr, 1)
        } else {
            0
        };
    }

    /// Row within the current BG/window tile (0..7), honouring CGB Y-flip.
    fn bg_row_in_tile(&self) -> u16 {
        let py = if self.fetcher.fetching_window {
            self.wly
        } else {
            self.scy.wrapping_add(self.ly)
        };
        let row = (py % 8) as u16;
        let attr = Attributes::from_bits_retain(self.fetcher.tile_attr);
        if self.mode == GBMode::CGB && attr.contains(Attributes::Y_FLIP) {
            7 - row
        } else {
            row
        }
    }

    /// Address + bank of the current BG/window tile's data row.
    fn bg_tile_data_addr(&self) -> (u16, usize) {
        let base = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
            0x8000
        } else {
            0x8800
        };
        let offset = if self.lcdc.contains(LCDC::TILE_DATA_AREA) {
            self.fetcher.tile_id as i16
        } else {
            (self.fetcher.tile_id as i8) as i16 + 128
        } as u16
            * 16;
        let attr = Attributes::from_bits_retain(self.fetcher.tile_attr);
        let bank = if self.mode == GBMode::CGB && attr.contains(Attributes::BANK) {
            1
        } else {
            0
        };
        (base + offset + self.bg_row_in_tile() * 2, bank)
    }

    fn bg_fetch_plane(&self, plane: u16) -> u8 {
        let (addr, bank) = self.bg_tile_data_addr();
        self.read_vram(addr + plane, bank)
    }

    /// Turn the two latched bit-planes into 8 BG pixels, index 0 = leftmost.
    fn assemble_bg_row(&self) -> [BgPixel; 8] {
        let attr = Attributes::from_bits_retain(self.fetcher.tile_attr);
        let xflip = self.mode == GBMode::CGB && attr.contains(Attributes::X_FLIP);
        let mut row = [BgPixel::default(); 8];
        for (i, cell) in row.iter_mut().enumerate() {
            let bit = if xflip { i as u8 } else { 7 - i as u8 };
            let lo = (self.fetcher.tile_low >> bit) & 1;
            let hi = (self.fetcher.tile_high >> bit) & 1;
            *cell = BgPixel {
                color: (hi << 1) | lo,
                cgb_attr: self.fetcher.tile_attr,
            };
        }
        row
    }

    /// Emit (or discard) one pixel from the head of the BG FIFO.
    fn shift_pixel(&mut self) {
        let bg = match self.bg_fifo.pop() {
            Some(p) => p,
            None => return,
        };

        // Discard pending pixels: SCX%8 fine scroll at line start, or the
        // left-edge clip of a WX<7 window. (A mid-line window trigger requires
        // the fine-scroll discard to have finished, so the two uses of this
        // counter can never overlap.)
        if self.shifter.discard > 0 {
            self.shifter.discard -= 1;
            return;
        }

        let x = self.shifter.emitted as usize;
        let obj = self.obj_line[x];
        self.emit_pixel(x, bg, obj);
        self.shifter.emitted += 1;
    }

    /// Resolve BG-vs-OBJ priority for one pixel and write it.
    fn emit_pixel(&mut self, x: usize, bg: BgPixel, obj: ObjPixel) {
        let bg_attr = Attributes::from_bits_retain(bg.cgb_attr);
        let bg_disabled = !self.use_cgb_mode() && !self.lcdc.contains(LCDC::WINDOW_PRIORITY);
        let bg_color = if bg_disabled { 0 } else { bg.color };

        let prio = if bg_color == 0 {
            Priority::Color0
        } else if bg_attr.contains(Attributes::PRIORITY) {
            Priority::Priority
        } else {
            Priority::Normal
        };

        let obj_present = obj.color != 0 && self.lcdc.contains(LCDC::OBJ_ENABLE);
        let bg_wins = if !obj_present {
            true
        } else {
            match self.mode {
                GBMode::CGB => {
                    if self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                        if prio == Priority::Color0 {
                            false
                        } else if !obj.bg_prio && prio != Priority::Priority {
                            false
                        } else {
                            true
                        }
                    } else {
                        obj.bg_prio && prio != Priority::Color0
                    }
                }
                GBMode::DMG => obj.bg_prio && prio != Priority::Color0,
            }
        };

        if bg_wins {
            self.emit_bg(x, bg_color, bg_attr, bg_disabled);
        } else {
            self.emit_obj(x, obj);
        }
    }

    fn emit_bg(&mut self, x: usize, color: u8, attr: Attributes, bg_disabled: bool) {
        if self.mode == GBMode::CGB {
            if bg_disabled {
                self.set_rgb_mapped(x, 0x7FFF);
                return;
            }
            let palette_no = if self.use_cgb_mode() {
                (attr.bits() & 0b0000_0111) as usize
            } else {
                0
            };
            let final_color = if !self.use_cgb_mode() {
                ((self.bgp >> (color * 2)) & 0x03) as usize
            } else {
                color as usize
            };
            let pa = palette_no * 8 + final_color * 2;
            let c = (self.bcpd[pa] as u16) | ((self.bcpd[pa + 1] as u16) << 8) & 0x7FFF;
            self.set_rgb_mapped(x, c);
        } else {
            // The pixel emitted on the dot a BGP write lands shows old|new.
            let pal = self.bgp_glitch.unwrap_or(self.bgp);
            let c = Self::grey_to_l(self.ppu_config.palette, pal, color as usize);
            self.framebuffer
                .set_pixel(c.r(), c.g(), c.b(), x, self.ly as usize);
        }
    }

    fn emit_obj(&mut self, x: usize, obj: ObjPixel) {
        if self.mode == GBMode::CGB {
            let palette_no = if self.use_cgb_mode() {
                (obj.cgb_attr & 0b0000_0111) as usize
            } else if obj.palette {
                1
            } else {
                0
            };
            let final_color = if !self.use_cgb_mode() {
                let obp = if obj.palette { self.obp1 } else { self.obp0 };
                ((obp >> (obj.color * 2)) & 0x03) as usize
            } else {
                obj.color as usize
            };
            let pa = palette_no * 8 + final_color * 2;
            let c = (self.ocpd[pa] as u16) | ((self.ocpd[pa + 1] as u16) << 8) & 0x7FFF;
            self.set_rgb_mapped(x, c);
        } else {
            let c = if obj.palette {
                Self::grey_to_l(self.ppu_config.palette, self.obp1, obj.color as usize)
            } else {
                Self::grey_to_l(self.ppu_config.palette, self.obp0, obj.color as usize)
            };
            self.framebuffer
                .set_pixel(c.r(), c.g(), c.b(), x, self.ly as usize);
        }
    }

    /// Begin a sprite fetch if a selected sprite is due at the current position
    /// and the BG FIFO has produced a tile to align against.
    fn try_start_sprite(&mut self) {
        // DMG suppresses OBJ fetches entirely when objects are disabled;
        // CGB still fetches (only mixing checks LCDC), so this shortens Mode 3
        // on DMG only.
        if self.mode == GBMode::DMG && !self.lcdc.contains(LCDC::OBJ_ENABLE) {
            return;
        }
        if self.bg_fifo.is_empty() {
            return;
        }

        let target = self.shifter.emitted as i16;
        let mut chosen: Option<usize> = None;
        for (i, s) in self.sprites.iter().enumerate() {
            if s.fetched {
                continue;
            }

            // A sprite is due once the pixel about to be emitted has reached
            // its screen X (x-8); left-clipped sprites (x<8) fire at x=0.
            if (s.x as i16 - 8) <= target {
                match chosen {
                    None => chosen = Some(i),
                    Some(j) => {
                        let o = &self.sprites[j];
                        if s.x < o.x || (s.x == o.x && s.oam_index < o.oam_index) {
                            chosen = Some(i);
                        }
                    }
                }
            }
        }

        if let Some(i) = chosen {
            self.sprites[i].fetched = true;
            self.fetcher.sprite = Some(self.sprites[i]);
            self.fetcher.sprite_step = FetchStep::TileId;
            self.fetcher.sprite_substep = 0;
        }
    }

    /// One dot of an in-progress sprite fetch (6 dots total, no Push step).
    fn sprite_fetch_step(&mut self) {
        let s = match self.fetcher.sprite {
            Some(s) => s,
            None => return,
        };
        match self.fetcher.sprite_step {
            FetchStep::TileId => {
                // Tile number came from OAM; this access reads nothing new.
                if self.fetcher.sprite_substep == 1 {
                    self.fetcher.sprite_step = FetchStep::TileLow;
                }
                self.fetcher.sprite_substep ^= 1;
            }
            FetchStep::TileLow => {
                if self.fetcher.sprite_substep == 1 {
                    self.fetcher.sprite_low = self.sprite_plane(&s, 0);
                    self.fetcher.sprite_step = FetchStep::TileHigh;
                }
                self.fetcher.sprite_substep ^= 1;
            }
            FetchStep::TileHigh => {
                if self.fetcher.sprite_substep == 1 {
                    self.fetcher.sprite_high = self.sprite_plane(&s, 1);
                    self.mix_sprite(&s);
                    self.fetcher.sprite = None; // Resume shifting next dot
                }
                self.fetcher.sprite_substep ^= 1;
            }
            FetchStep::Push => self.fetcher.sprite = None,
        }
    }

    fn oam_top(s: &SelectedSprite) -> u8 {
        s.y.wrapping_sub(16)
    }

    fn sprite_plane(&self, s: &SelectedSprite, plane: u16) -> u8 {
        let size: u8 = if self.lcdc.contains(LCDC::OBJ_SIZE) {
            16
        } else {
            8
        };
        let mut row = self.ly.wrapping_sub(Self::oam_top(s)) & (size - 1);

        if s.attr.contains(Attributes::Y_FLIP) {
            row = size - 1 - row;
        }

        let tile = if size == 16 { s.tile & 0xFE } else { s.tile };
        let addr = 0x8000u16 + tile as u16 * 16 + row as u16 * 2;
        let bank = if self.mode == GBMode::CGB && s.attr.contains(Attributes::BANK) {
            1
        } else {
            0
        };

        self.read_vram(addr + plane, bank)
    }

    /// Mix a freshly-fetched sprite into the per-line OBJ overlay. The first
    /// (highest-priority) sprite to cover a pixel keeps it, matching the OBJ
    /// FIFO's "don't overwrite a non-transparent pixel" rule.
    fn mix_sprite(&mut self, s: &SelectedSprite) {
        let base = s.x as i16 - 8;
        let xflip = s.attr.contains(Attributes::X_FLIP);
        let palette = s.attr.contains(Attributes::PALETTE_NO_0);
        let bg_prio = s.attr.contains(Attributes::PRIORITY);
        for i in 0..8i16 {
            let sx = base + i;
            if sx < 0 || sx >= SCREEN_W as i16 {
                continue;
            }

            let bit = if xflip { i as u8 } else { 7 - i as u8 };
            let lo = (self.fetcher.sprite_low >> bit) & 1;
            let hi = (self.fetcher.sprite_high >> bit) & 1;
            let color = (hi << 1) | lo;
            if color == 0 {
                continue;
            }

            let slot = &mut self.obj_line[sx as usize];
            if slot.color != 0 {
                continue; // An earlier sprite already owns this pixel
            }

            *slot = ObjPixel {
                color,
                palette,
                bg_prio,
                cgb_attr: s.attr.bits(),
            };
        }
    }

    fn check_window_trigger(&mut self) {
        if self.window_active || self.fetcher.in_sprite_fetch() {
            return;
        }

        if !self.lcdc.contains(LCDC::WINDOW_ENABLE) || !self.window_y_condition {
            return;
        }

        // The WX<7 + fine-scroll (SCX%8 != 0) glitch and the WX=166 quirk
        // are not yet modelled.
        if self.shifter.discard != 0 {
            return;
        }

        // WX >= 167 never starts a window (documented); the guard is needed
        // explicitly because the position counter wraps up from its -1 start.
        if self.wx >= 167 {
            return;
        }

        if self.shifter.pos == self.wx {
            self.window_active = true;
            self.window_line_active = true;
            self.fetcher.fetching_window = true;
            self.fetcher.tile_x = 0;
            self.fetcher.done_dummy = true; // No warm-up fetch on window restart
            self.fetcher.restart_bg();
            self.bg_fifo.clear();

            // WX 0..6: the window still starts at the screen's left edge, but
            // its first 7-WX pixels fall off the left side and are clipped.
            if self.wx < 7 {
                self.shifter.discard = 7 - self.wx;
            }
        }
    }

    fn inc_ly(&mut self) {
        // The window's internal line counter advances once per line on which
        // the window actually activated during Mode 3 (tracked by
        // `window_line_active`, latched when the trigger fires), never from a
        // fresh LCDC read: a line where WIN_EN is toggled off before HBlank
        // must not advance it even though the enable bit reads high now
        // (m2_win_en_toggle). The latch is consumed here so VBlank lines (no
        // Mode 3, no re-latch) never advance it.
        if self.window_line_active {
            self.wly += 1;
        }
        self.window_line_active = false;

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

    pub fn disable_boot_rom(&mut self) {
        self.boot_rom_enabled = false;
    }

    /// Drain the interrupt requests and the HBlank edge produced since the last
    /// call, clearing them. Used by the peer-chip bus to build its `Ticked`
    /// result instead of reaching into the public fields.
    pub fn take_events(&mut self) -> (u8, bool) {
        let irq = self.interrupts.bits();
        self.interrupts = Interrupts::empty();
        let hblank = self.entered_hblank;
        self.entered_hblank = false;
        (irq, hblank)
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

    /// Unconditional VRAM write at the current bank, for the HDMA/GPDMA engine
    /// (which drives VRAM directly, not through the CPU's mode-gated port).
    pub fn write_vram_direct(&mut self, a: u16, v: u8) {
        let bank = self.vram_bank;
        self.write_vram(a, v, bank);
    }

    pub fn oam_corrupt_inc(&mut self) {
        // DMG/SGB OAM corruption: an address in FE00-FEFF held by a 16-bit
        // inc/dec during mode 2 glitches the row the PPU is scanning. The
        // corruption sources from the previous row, so only row 0 is immune.
        // OAM is a 16-bit-word bus; corruption acts on words.
        if self.mode != GBMode::DMG || self.ppu_mode != PPUMode::OAMScan {
            return;
        }
        let row = (self.cycle_count / 4) as usize; // One row scanned per M-cycle
        if row < 1 || row >= 20 {
            return;
        }

        let word = |o: &[u8; 0xA0], r: usize, w: usize| -> u16 {
            let i = r * 8 + w * 2;
            (o[i] as u16) | ((o[i + 1] as u16) << 8)
        };
        let a = word(&self.oam, row, 0);
        let b = word(&self.oam, row - 1, 0);
        let c = word(&self.oam, row - 1, 2);
        let new0 = ((a ^ c) & (b ^ c)) ^ c;

        let base = row * 8;
        self.oam[base] = new0 as u8;
        self.oam[base + 1] = (new0 >> 8) as u8;
        for w in 1..4 {
            self.oam[base + w * 2] = self.oam[(row - 1) * 8 + w * 2];
            self.oam[base + w * 2 + 1] = self.oam[(row - 1) * 8 + w * 2 + 1];
        }
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
            0xFF4F => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    0xFE | self.vram_bank as u8
                }
            }
            0xFF68 => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    self.bcps.read() | 0x40
                }
            }
            0xFF69 => {
                if self.mode == GBMode::DMG || self.ppu_mode == PPUMode::Draw {
                    0xFF
                } else {
                    self.bcpd[self.bcps.address as usize]
                }
            }
            0xFF6A => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    self.ocps.read() | 0x40
                }
            }
            0xFF6B => {
                if self.mode == GBMode::DMG || self.ppu_mode == PPUMode::Draw {
                    0xFF
                } else {
                    self.ocpd[self.ocps.address as usize]
                }
            }
            0xFF6C => 0xFE | self.opri as u8,
            _ => 0xFF,
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
                let was_on = self.lcdc.contains(LCDC::LCD_ENABLE);
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
                } else if !was_on {
                    // Enabling the LCD restarts at the beginning of scanline 0.
                    // The seed sets the permanent phase between the PPU's dot
                    // counter and the CPU's M-cycle grid (a line is 456 ≡ 0 mod
                    // 4 dots, so it never re-aligns). The value encodes the
                    // hardware phase pinned by the mealybug m3 suite; the
                    // first-frame enable quirks (short line 0, mode-0 readback)
                    // are not yet modelled.
                    self.reset_ly();
                    self.ppu_mode = PPUMode::OAMScan;
                    self.cycle_count = 5;
                }
            }
            0xFF41 => {
                let sanitised = v & 0b1111_1000 | (self.lcds.bits() & 0b0000_0100);
                self.lcds = LCDS::from_bits_truncate(sanitised);
                self.check_lyc();
            }
            0xFF42 => self.scy = v,
            0xFF43 => self.scx = v,
            0xFF44 => {} // LY is read-only; writes are ignored by hardware
            0xFF45 => {
                self.lc = v;
                self.check_lyc();
            }
            0xFF47 => {
                // DMG: a BGP write landing during Mode 3 leaves `old | new`
                // visible for the single dot the write occupies before the new
                // value takes over — the 1-px slivers the mealybug
                // m3_bgp_change reference pins down. CGB does not glitch.
                if self.mode == GBMode::DMG && self.ppu_mode == PPUMode::Draw {
                    self.bgp_glitch = Some(self.bgp | v);
                }
                self.bgp = v;
            }
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
