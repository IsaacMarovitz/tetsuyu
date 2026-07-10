//! The Mode-3 pixel pipeline: the BG/window tile fetcher, the BG and OBJ
//! FIFOs, the pixel shifter, and the sprite fetch/penalty machinery

use crate::components::mode::GBMode;
use crate::components::ppu::structs::*;

#[inline]
fn rd(vram: &[u8], a: u16, bank: usize) -> u8 {
    vram[bank * 0x2000 + (a as usize - 0x8000)]
}

/// A background pixel queued in the BG FIFO.
#[derive(Copy, Clone, Default)]
pub struct BgPixel {
    pub color: u8,
    pub cgb_attr: u8,
}

/// A sprite pixel queued in the OBJ FIFO.
#[derive(Copy, Clone, Default)]
pub struct ObjPixel {
    pub color: u8,
    pub palette: bool,
    pub bg_prio: bool,
    pub cgb_attr: u8,
    pub oam_index: u8,
}

/// A sprite chosen during the Mode 2 OAM scan (up to 10 per line).
#[derive(Copy, Clone)]
pub struct SelectedSprite {
    pub oam_index: u8,
    pub x: u8,
    pub y: u8,
    pub tile: u8,
    pub attr: Attributes,
    pub fetched: bool,
}

/// Register snapshot handed to the pipeline for one dot / one line start.
#[derive(Copy, Clone)]
pub struct Regs {
    pub mode: GBMode,
    pub lcdc: LCDC,
    pub scx: u8,
    pub scy: u8,
    pub ly: u8,
    pub wx: u8,
    pub wly: u8,
    pub window_y: bool,
    pub opri: bool,
}

/// One emitted pixel: screen column plus the BG and OBJ pixels to resolve.
pub struct Emit {
    pub x: usize,
    pub bg: BgPixel,
    pub obj: ObjPixel,
}

/// BG pixel FIFO, up to two tiles deep so the fetcher runs a tile ahead of
/// the shifter.
struct BgFifo {
    data: [BgPixel; 16],
    head: usize,
    len: usize,
}

impl BgFifo {
    fn new() -> Self {
        Self { data: [BgPixel::default(); 16], head: 0, len: 0 }
    }
    fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
    fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn push(&mut self, p: BgPixel) {
        let t = (self.head + self.len) % 16;
        self.data[t] = p;
        self.len += 1;
    }
    fn pop(&mut self) -> Option<BgPixel> {
        if self.len == 0 {
            return None;
        }
        let v = self.data[self.head];
        self.head = (self.head + 1) % 16;
        self.len -= 1;
        Some(v)
    }
}

/// OBJ FIFO: an 8-slot shift register aligned to the pixel about to be
/// emitted. A sprite fetch overlays its 8 pixels; each emitted BG pixel shifts
/// one OBJ pixel out in lockstep, backfilling transparent.
struct ObjFifo {
    data: [ObjPixel; 8],
    head: usize,
}

impl ObjFifo {
    fn new() -> Self {
        Self { data: [ObjPixel::default(); 8], head: 0 }
    }
    fn clear(&mut self) {
        self.data = [ObjPixel::default(); 8];
        self.head = 0;
    }
    fn shift_out(&mut self) -> ObjPixel {
        let p = self.data[self.head];
        self.data[self.head] = ObjPixel::default();
        self.head = (self.head + 1) % 8;
        p
    }
    fn slot(&mut self, pos: usize) -> &mut ObjPixel {
        &mut self.data[(self.head + pos) % 8]
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FetchStep {
    Tile,
    Low,
    High,
}

#[derive(Copy, Clone)]
enum SpriteStage {
    Align,
    Core(u8),
}

#[derive(Copy, Clone)]
struct SpriteFetch {
    sprite: SelectedSprite,
    offset: i16,
    stage: SpriteStage,
}

pub struct Pipeline {
    // BG fetcher
    step: FetchStep,
    sub: u8,
    tile_x: u8,
    fetching_window: bool,
    tile_id: u8,
    tile_attr: u8,
    tile_low: u8,
    tile_high: u8,
    /// A completed tile waiting for the shifter to drain enough to load it.
    pending: Option<[BgPixel; 8]>,
    // FIFOs
    bg: BgFifo,
    obj: ObjFifo,
    // pixel clock (SACU)
    /// PX — the single horizontal counter, clocked by the pixel clock. Starts at
    /// 0; visible pixels are gated at PX >= 9 (screen_x = px - 9), so the 8
    /// warm-up shifts (PX 1-8) drain the junk row. Window matches `px == WX`,
    /// objects `px >= SPRITEX + 1`. Terminal at 167.
    px: i16,
    /// POKY: the pixel clock is held until the line's first BG tile is fetched.
    data_ready: bool,
    /// ROXY fine-scroll gate: holds the pixel clock for SCX&7 dots at startup.
    fine: u8,
    /// Window left-edge clip (WX<7): pixels shifted but not emitted.
    clip: u8,
    // window
    window_active: bool,
    window_triggered: bool,
    // sprites
    sprites: Vec<SelectedSprite>,
    sprite_fetch: Option<SpriteFetch>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            step: FetchStep::Tile,
            sub: 0,
            tile_x: 0,
            fetching_window: false,
            tile_id: 0,
            tile_attr: 0,
            tile_low: 0,
            tile_high: 0,
            pending: None,
            bg: BgFifo::new(),
            obj: ObjFifo::new(),
            px: 0,
            data_ready: false,
            fine: 0,
            clip: 0,
            window_active: false,
            window_triggered: false,
            sprites: Vec::with_capacity(10),
            sprite_fetch: None,
        }
    }

    /// Reset all pipeline state for a new Mode 3 and select this line's sprites.
    pub fn start_line(&mut self, oam: &[u8], r: Regs) {
        self.step = FetchStep::Tile;
        self.sub = 0;
        self.tile_x = 0;
        self.fetching_window = false;
        self.pending = None;
        self.bg.clear();
        self.obj.clear();
        // Junk-row fill: prime the shifter with a discarded tile (drained across
        // the PX 1-8 warm-up) so the fetcher just runs — no dummy-fetch patch
        // state. The pixel clock stays held (POKY) until the first real tile is
        // fetched.
        for _ in 0..8 {
            self.bg.push(BgPixel::default());
        }
        self.px = 0;
        self.data_ready = false;
        self.fine = 0;
        self.clip = 0;
        self.window_active = false;
        self.window_triggered = false;
        self.sprite_fetch = None;
        self.select_sprites(oam, r);
    }

    /// The window activated during this Mode 3 (latched, for the WLY counter).
    pub fn window_triggered(&self) -> bool {
        self.window_triggered
    }

    /// All 160 visible pixels have been emitted (Mode 3 is over). PX terminal is
    /// 167 (WODU); screen pixel 159 is the last, captured as PX reaches it.
    pub fn finished(&self) -> bool {
        self.px >= 168
    }

    fn select_sprites(&mut self, oam: &[u8], r: Regs) {
        self.sprites.clear();
        let size = if r.lcdc.contains(LCDC::OBJ_SIZE) { 16 } else { 8 };
        let line = r.ly as i32 + 16;
        for i in 0..40u8 {
            let o = i as usize * 4;
            let y = oam[o] as i32;
            if line >= y && line < y + size {
                self.sprites.push(SelectedSprite {
                    oam_index: i * 4,
                    x: oam[o + 1],
                    y: oam[o],
                    tile: oam[o + 2],
                    attr: Attributes::from_bits_retain(oam[o + 3]),
                    fetched: false,
                });
                if self.sprites.len() == 10 {
                    break;
                }
            }
        }
    }

    /// Advance the pipeline one dot. Returns a pixel to paint if one was
    /// emitted this dot.
    pub fn tick(&mut self, vram: &[u8], r: Regs) -> Option<Emit> {
        self.check_window(r);

        // A sprite fetch freezes the pixel clock. `Align` advances the BG
        // fetcher (without loading — the assembled tile is held) until it
        // reaches a tile boundary. `Core` is the  frozen 6-dot data fetch;
        // its final dot overlays and falls through to a normal shift.
        if let Some(mut sf) = self.sprite_fetch {
            match sf.stage {
                SpriteStage::Align => {
                    self.bg_fetch(vram, r, false);
                    if self.pending.is_some() && !self.bg.is_empty() {
                        // The fetch-complete dot is the first core dot, not a
                        // separate transition dot — enter at Core(1).
                        sf.stage = SpriteStage::Core(1);
                    }
                    self.sprite_fetch = Some(sf);
                    return None;
                }
                SpriteStage::Core(c) => {
                    if c >= 5 {
                        self.overlay_sprite(vram, r, &sf.sprite, sf.offset);
                        self.sprite_fetch = None;
                        // A stacked same-position object chains here — before
                        // bg_fetch loads the held tile and before any shift — so
                        // the fetch-complete condition stays satisfied and it
                        // costs exactly its 6-dot core (no fresh alignment).
                        if self.try_start_sprite(r) {
                            return None;
                        }
                    } else {
                        sf.stage = SpriteStage::Core(c + 1);
                        self.sprite_fetch = Some(sf);
                        return None;
                    }
                }
            }
        }

        self.bg_fetch(vram, r, true);

        if self.try_start_sprite(r) {
            return None;
        }

        // The pixel clock (SACU). It halts — PX unchanged, nothing emitted — on
        // any of: the sprite freeze (handled above), POKY (first tile not yet
        // fetched), the ROXY fine-scroll gate (SCX&7 dots), or an empty shifter.
        // Otherwise PX advances one step this dot.
        if !self.data_ready {
            return None;
        }
        if self.fine < (r.scx & 7) {
            self.fine += 1;
            return None;
        }
        if self.bg.is_empty() {
            return None;
        }

        let bg = self.bg.pop().unwrap();
        let obj = self.obj.shift_out();
        self.px += 1;

        // WX<7 left-edge clip: the pixel shifted but lies off the screen's left.
        if self.clip > 0 {
            self.clip -= 1;
            return None;
        }

        // PX 1-8 are the warm-up (the junk row) — shifted but never reaching
        // glass. Visible output is gated at PX >= 9; screen_x = px - 9.
        if self.px >= 9 {
            let x = (self.px - 9) as usize;
            if x < 160 {
                return Some(Emit { x, bg, obj });
            }
        }
        None
    }

    /// One dot of the BG/window fetch. Three 2-dot accesses assemble a tile
    /// into `pending`; it loads into the FIFO once the shifter has drained to
    /// half (the 8-dot cadence's tail), then the fetcher restarts on the next
    /// column.
    fn bg_fetch(&mut self, vram: &[u8], r: Regs, can_load: bool) {
        if let Some(row) = self.pending {
            // Load exactly when the 8-pixel shifter has drained (the drain
            // detector is clocked by the pixel clock), locking the tile cycle
            // to 8 dots: 6 to assemble + 2 held here. `pending` is the temp
            // latch, the FIFO the single 8-pixel shifter.
            if can_load && self.bg.is_empty() {
                for p in row {
                    self.bg.push(p);
                }
                self.tile_x = self.tile_x.wrapping_add(1);
                self.pending = None;
                self.step = FetchStep::Tile;
                self.sub = 0;
            }
            // Whether we loaded, are waiting for room, or are holding the tile
            // through a sprite fetch's alignment, no VRAM access this dot.
            return;
        }

        match self.step {
            FetchStep::Tile => {
                if self.sub == 0 {
                    self.sub = 1;
                } else {
                    self.sub = 0;
                    self.fetch_tile_id(vram, r);
                    self.step = FetchStep::Low;
                }
            }
            FetchStep::Low => {
                if self.sub == 0 {
                    self.sub = 1;
                } else {
                    self.sub = 0;
                    self.tile_low = self.fetch_plane(vram, r, 0);
                    self.step = FetchStep::High;
                }
            }
            FetchStep::High => {
                if self.sub == 0 {
                    self.sub = 1;
                } else {
                    self.sub = 0;
                    self.tile_high = self.fetch_plane(vram, r, 1);
                    self.pending = Some(self.assemble_row(r));
                    // POKY: the first completed BG fetch releases the pixel clock.
                    self.data_ready = true;
                }
            }
        }
    }

    fn fetch_tile_id(&mut self, vram: &[u8], r: Regs) {
        let (base, tx, ty) = if self.fetching_window {
            let base = if r.lcdc.contains(LCDC::WINDOW_AREA) { 0x9C00 } else { 0x9800 };
            (base, self.tile_x as u16 & 31, (r.wly as u16 >> 3) & 31)
        } else {
            let base = if r.lcdc.contains(LCDC::BG_TILE_MAP_AREA) { 0x9C00 } else { 0x9800 };
            let tx = ((r.scx as u16 / 8) + self.tile_x as u16) & 31;
            let ty = ((r.scy.wrapping_add(r.ly) as u16) >> 3) & 31;
            (base, tx, ty)
        };

        let addr = base + ty * 32 + tx;
        self.tile_id = rd(vram, addr, 0);
        self.tile_attr = if r.mode == GBMode::CGB { rd(vram, addr, 1) } else { 0 };
    }

    fn row_in_tile(&self, r: Regs) -> u16 {
        let py = if self.fetching_window { r.wly } else { r.scy.wrapping_add(r.ly) };
        let row = (py % 8) as u16;
        let attr = Attributes::from_bits_retain(self.tile_attr);

        if r.mode == GBMode::CGB && attr.contains(Attributes::Y_FLIP) {
            7 - row
        } else {
            row
        }
    }

    fn tile_data_addr(&self, r: Regs) -> (u16, usize) {
        let base = if r.lcdc.contains(LCDC::TILE_DATA_AREA) { 0x8000 } else { 0x8800 };
        let offset = if r.lcdc.contains(LCDC::TILE_DATA_AREA) {
            self.tile_id as i16
        } else {
            (self.tile_id as i8) as i16 + 128
        } as u16
            * 16;

        let attr = Attributes::from_bits_retain(self.tile_attr);
        let bank = if r.mode == GBMode::CGB && attr.contains(Attributes::BANK) { 1 } else { 0 };

        (base + offset + self.row_in_tile(r) * 2, bank)
    }

    fn fetch_plane(&self, vram: &[u8], r: Regs, plane: u16) -> u8 {
        let (addr, bank) = self.tile_data_addr(r);
        rd(vram, addr + plane, bank)
    }

    fn assemble_row(&self, r: Regs) -> [BgPixel; 8] {
        let attr = Attributes::from_bits_retain(self.tile_attr);
        let xflip = r.mode == GBMode::CGB && attr.contains(Attributes::X_FLIP);
        let mut row = [BgPixel::default(); 8];

        for (i, cell) in row.iter_mut().enumerate() {
            let bit = if xflip { i as u8 } else { 7 - i as u8 };
            let lo = (self.tile_low >> bit) & 1;
            let hi = (self.tile_high >> bit) & 1;
            *cell = BgPixel { color: (hi << 1) | lo, cgb_attr: self.tile_attr };
        }

        row
    }

    /// Begin a sprite fetch if a selected object is due at the current emit
    /// position (BG FIFO non-empty to align against). Returns whether one
    /// started (which freezes the pixel clock this dot).
    fn try_start_sprite(&mut self, r: Regs) -> bool {
        if r.mode == GBMode::DMG && !r.lcdc.contains(LCDC::OBJ_ENABLE) {
            return false;
        }

        if self.bg.is_empty() {
            return false;
        }

        let mut chosen: Option<usize> = None;
        for (i, s) in self.sprites.iter().enumerate() {
            if s.fetched {
                continue;
            }
            if (s.x as i16 + 1) <= self.px {
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
            let s = self.sprites[i];
            let offset = (s.x as i16 + 1) - self.px;
            // If the BG fetch is already complete (tile held in `pending`), the
            // trigger and fetch-complete coincide on this dot — it is core dot 1,
            // not a wasted alignment dot.
            let stage = if self.pending.is_some() {
                SpriteStage::Core(0)
            } else {
                SpriteStage::Align
            };
            self.sprite_fetch = Some(SpriteFetch { sprite: s, offset, stage });
            return true;
        }
        false
    }

    fn overlay_sprite(&mut self, vram: &[u8], r: Regs, s: &SelectedSprite, offset: i16) {
        let (lo, hi) = self.sprite_planes(vram, r, s);
        let xflip = s.attr.contains(Attributes::X_FLIP);

        // OAM-index priority applies only on CGB with OPRI cleared; DMG and
        // OPRI=1 resolve overlaps by fetch (coordinate) order.
        let index_priority = r.mode == GBMode::CGB && !r.opri;

        for i in 0..8i16 {
            let pos = offset + i;
            if !(0..8).contains(&pos) {
                continue;
            }
            let bit = if xflip { i as u8 } else { 7 - i as u8 };
            let color = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
            if color == 0 {
                continue;
            }
            let slot = self.obj.slot(pos as usize);
            if slot.color != 0 && !(index_priority && s.oam_index < slot.oam_index) {
                continue;
            }
            *slot = ObjPixel {
                color,
                palette: s.attr.contains(Attributes::PALETTE_NO_0),
                bg_prio: s.attr.contains(Attributes::PRIORITY),
                cgb_attr: s.attr.bits(),
                oam_index: s.oam_index,
            };
        }
    }

    fn sprite_planes(&self, vram: &[u8], r: Regs, s: &SelectedSprite) -> (u8, u8) {
        let size: u8 = if r.lcdc.contains(LCDC::OBJ_SIZE) { 16 } else { 8 };
        let top = s.y.wrapping_sub(16);
        let mut row = r.ly.wrapping_sub(top) & (size - 1);

        if s.attr.contains(Attributes::Y_FLIP) {
            row = size - 1 - row;
        }

        let tile = if size == 16 { s.tile & 0xFE } else { s.tile };
        let addr = 0x8000u16 + tile as u16 * 16 + row as u16 * 2;
        let bank = if r.mode == GBMode::CGB && s.attr.contains(Attributes::BANK) { 1 } else { 0 };

        (rd(vram, addr, bank), rd(vram, addr + 1, bank))
    }

    fn check_window(&mut self, r: Regs) {
        if self.window_active || self.sprite_fetch.is_some() {
            return;
        }

        if !r.lcdc.contains(LCDC::WINDOW_ENABLE) || !r.window_y {
            return;
        }

        if r.wx >= 167 {
            return;
        }

        if self.px == r.wx as i16 {
            self.window_active = true;
            self.window_triggered = true;
            self.fetching_window = true;
            self.tile_x = 0;
            self.pending = None;
            self.step = FetchStep::Tile;
            self.sub = 0;
            self.bg.clear();
            if r.wx < 7 {
                self.clip = 7 - r.wx;
            }
        }
    }
}
