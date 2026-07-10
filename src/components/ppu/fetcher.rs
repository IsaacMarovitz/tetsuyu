//! The Mode-3 pixel pipeline: the BG/window tile fetcher, the BG and OBJ
//! FIFOs, the pixel shifter, and the sprite fetch/penalty machinery

use crate::components::mode::GBMode;
use crate::components::ppu::structs::*;

const W: usize = 160;

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
    fn len(&self) -> usize {
        self.len
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
    /// The line's first BG fetch is discarded (hardware warm-up) to restore the
    /// ~6-dot Mode 3 startup latency; set once that discard fetch completes.
    /// Forced true on a window restart, which has no warm-up.
    warmed: bool,
    tile_id: u8,
    tile_attr: u8,
    tile_low: u8,
    tile_high: u8,
    /// A completed tile waiting for the shifter to drain enough to load it.
    pending: Option<[BgPixel; 8]>,
    // FIFOs
    bg: BgFifo,
    obj: ObjFifo,
    // pixel clock
    emitted: u8,
    discard: u8,
    // window
    window_active: bool,
    window_triggered: bool,
    win_pos: u8,
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
            warmed: false,
            tile_id: 0,
            tile_attr: 0,
            tile_low: 0,
            tile_high: 0,
            pending: None,
            bg: BgFifo::new(),
            obj: ObjFifo::new(),
            emitted: 0,
            discard: 0,
            window_active: false,
            window_triggered: false,
            win_pos: 251,
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
        self.warmed = false;
        self.pending = None;
        self.bg.clear();
        self.obj.clear();
        self.emitted = 0;
        self.discard = r.scx % 8;
        self.window_active = false;
        self.window_triggered = false;
        self.win_pos = 251;
        self.sprite_fetch = None;
        self.select_sprites(oam, r);
    }

    /// The window activated during this Mode 3 (latched, for the WLY counter).
    pub fn window_triggered(&self) -> bool {
        self.window_triggered
    }

    /// All 160 visible pixels have been emitted (Mode 3 is over).
    pub fn finished(&self) -> bool {
        self.emitted as usize >= W
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
                        sf.stage = SpriteStage::Core(0);
                    }
                    self.sprite_fetch = Some(sf);
                    return None;
                }
                SpriteStage::Core(c) => {
                    if c >= 5 {
                        self.overlay_sprite(vram, r, &sf.sprite, sf.offset);
                        self.sprite_fetch = None;
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

        let emit = if self.bg.is_empty() { None } else { self.shift() };

        if self.sprite_fetch.is_none() {
            self.win_pos = self.win_pos.wrapping_add(1);
        }

        emit
    }

    /// Pop one pixel from the shifters, honouring the fine-scroll / window
    /// left-edge discard.
    fn shift(&mut self) -> Option<Emit> {
        let bg = self.bg.pop().unwrap();
        let obj = self.obj.shift_out();

        if self.discard > 0 {
            self.discard -= 1;
            return None;
        }

        let x = self.emitted as usize;
        self.emitted += 1;
        Some(Emit { x, bg, obj })
    }

    /// One dot of the BG/window fetch. Three 2-dot accesses assemble a tile
    /// into `pending`; it loads into the FIFO once the shifter has drained to
    /// half (the 8-dot cadence's tail), then the fetcher restarts on the next
    /// column.
    fn bg_fetch(&mut self, vram: &[u8], r: Regs, can_load: bool) {
        if let Some(row) = self.pending {
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
                    if !self.warmed {
                        // Hardware discards the line's first fetch and refetches
                        // the same column — the Mode 3 startup latency (~6 dots).
                        self.warmed = true;
                        self.step = FetchStep::Tile;
                    } else {
                        self.pending = Some(self.assemble_row(r));
                    }
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

        let target = self.emitted as i16;
        let mut chosen: Option<usize> = None;
        for (i, s) in self.sprites.iter().enumerate() {
            if s.fetched {
                continue;
            }
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
            let s = self.sprites[i];
            let offset = (s.x as i16 - 8) - self.emitted as i16;
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

        // The WX<7 + fine-scroll glitch and the WX=166 quirk are not modelled.
        if self.discard != 0 {
            return;
        }

        if r.wx >= 167 {
            return;
        }
        
        if self.win_pos == r.wx {
            self.window_active = true;
            self.window_triggered = true;
            self.fetching_window = true;
            self.tile_x = 0;
            self.pending = None;
            self.step = FetchStep::Tile;
            self.sub = 0;
            self.warmed = true;
            self.bg.clear();
            if r.wx < 7 {
                self.discard = 7 - r.wx;
            }
        }
    }
}
