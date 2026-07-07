//! This module holds the *data* for the sub-dot-accurate renderer — the two
//! pixel FIFOs, the background/window/sprite fetcher, and the LCD shifter. The
//! per-dot logic that drives them lives on `PPU` (in `ppu.rs`), so it can reach
//! the VRAM/OAM/register state and reuse the existing tile-address and colour
//! maths. Keeping the state here and the behaviour there avoids a
//! self-referential borrow of `PPU`.

use crate::components::ppu::structs::Attributes;

/// One step of the 2-dot background (or sprite) fetch cycle.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum FetchStep {
    TileId,
    TileLow,
    TileHigh,
    Push,
}

/// A background pixel queued in the BG FIFO.
#[derive(Copy, Clone, Default)]
pub struct BgPixel {
    /// Colour index 0..3 (pre-palette).
    pub color: u8,
    /// CGB BG attribute byte (palette 0..7 + priority + bank + flips). On DMG
    /// this stays 0 and only `color` is used.
    pub cgb_attr: u8,
}

/// A sprite pixel queued in the OBJ FIFO.
#[derive(Copy, Clone, Default)]
pub struct ObjPixel {
    /// Colour index 0..3; 0 = transparent.
    pub color: u8,
    /// DMG palette select: `false` = OBP0, `true` = OBP1.
    pub palette: bool,
    /// OBJ-to-BG priority bit (attribute bit 7): when set, BG colours 1..3 win.
    pub bg_prio: bool,
    /// CGB OBJ attribute byte. Unused on DMG.
    pub cgb_attr: u8,
    /// OAM byte index of the sprite that owns this pixel. Used to resolve
    /// overlaps under CGB OAM-priority mode (OPRI=0), where the lower-index
    /// object wins regardless of X.
    pub oam_index: u8,
}

/// A fixed-capacity (≤8) ring buffer of pixels.
pub struct PixelFifo<T: Copy + Default> {
    data: [T; 8],
    head: u8,
    len: u8,
}

impl<T: Copy + Default> PixelFifo<T> {
    pub fn new() -> Self {
        Self {
            data: [T::default(); 8],
            head: 0,
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Replace the whole FIFO with the 8 pixels of a freshly-assembled tile.
    /// Only valid when empty — the BG fetcher only pushes into an empty FIFO.
    pub fn push_row(&mut self, row: [T; 8]) {
        self.data = row;
        self.head = 0;
        self.len = 8;
    }

    /// Pop one pixel from the head.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let v = self.data[self.head as usize];
        self.head = (self.head + 1) % 8;
        self.len -= 1;
        Some(v)
    }
}

/// The background/window/sprite fetcher
pub struct Fetcher {
    pub step: FetchStep,
    /// 0 or 1 within the current 2-dot step; the access resolves on substep 1.
    pub substep: u8,
    /// Fetcher column within the current line's map row.
    pub tile_x: u8,
    pub fetching_window: bool,
    /// The dummy first fetch primes the pipeline and is discarded
    pub done_dummy: bool,
    // Latched bytes / attribute for the in-progress BG tile.
    pub tile_id: u8,
    pub tile_low: u8,
    pub tile_high: u8,
    pub tile_attr: u8,
    // Sprite sub-fetch state (runs to completion before the shifter resumes).
    pub sprite: Option<SelectedSprite>,
    pub sprite_low: u8,
    pub sprite_high: u8,
    /// Dots remaining in the current OBJ's Mode-3 penalty. The OBJ fetch stalls
    /// the shifter for this many dots (the Pandocs OBJ penalty: an optional
    /// per-tile alignment wait plus the flat 6-dot tile fetch). The pixels are
    /// mixed and the sprite cleared when it reaches 0.
    pub sprite_stall: u8,
}

impl Fetcher {
    pub fn new() -> Self {
        Self {
            step: FetchStep::TileId,
            substep: 0,
            tile_x: 0,
            fetching_window: false,
            done_dummy: false,
            tile_id: 0,
            tile_low: 0,
            tile_high: 0,
            tile_attr: 0,
            sprite: None,
            sprite_low: 0,
            sprite_high: 0,
            sprite_stall: 0,
        }
    }

    /// True while a sprite sub-fetch is in progress (shifter must not advance).
    pub fn in_sprite_fetch(&self) -> bool {
        self.sprite.is_some()
    }

    /// Restart the BG fetch cycle at `TileId` (window trigger / after a push).
    pub fn restart_bg(&mut self) {
        self.step = FetchStep::TileId;
        self.substep = 0;
    }
}

/// The LCD shifter
pub struct Shifter {
    /// Screen X of the next visible pixel to emit (0..160). Mode 3 ends at 160.
    pub emitted: u8,
    /// Fine-scroll pixels still to discard at line start (SCX % 8), reused for
    /// the left-edge clip of a WX<7 window.
    pub discard: u8,
    /// The window trigger counter (Pandocs "Window rendering criteria"):
    /// initialised to 0 at Mode 3 start, it increments 7 times before the first
    /// pixel is rendered (covering the fine-scroll discards) and then once per
    /// pixel, and the window activates on the dot it equals WX. So the pixel at
    /// screen X is reached when the counter is X + 7, putting the window's first
    /// column at screen `WX - 7`. Ticks every non-sprite-stalled Mode 3 dot;
    /// held while a sprite fetch stalls the shifter. Initialised to 251 (= -5)
    /// and ticked before the first comparison so the pre-render increments land
    /// the window's first pixel at `WX - 7`. Pinned to one-dot resolution by
    /// m3_window_timing's WX = LY sweep and m3_wx_*_change.
    pub pos: u8,
}

impl Shifter {
    pub fn new() -> Self {
        Self {
            emitted: 0,
            discard: 0,
            pos: 251,
        }
    }
}

/// A sprite chosen during the Mode 2 OAM scan, up to 10 per line.
#[derive(Copy, Clone)]
pub struct SelectedSprite {
    /// OAM byte index (0, 4, 8, …) — stable order + priority tie-break.
    pub oam_index: u8,
    /// OAM X (8 = leftmost fully on-screen).
    pub x: u8,
    /// OAM Y (top + 16).
    pub y: u8,
    pub tile: u8,
    pub attr: Attributes,
    /// Set once this sprite's fetch has been consumed, so it triggers once.
    pub fetched: bool,
}
