use super::bus::{BusDir, Chip, Pins, Ticked};
use crate::components::mode::GBMode;

/// The DMA engine: OAM DMA (0xFF46) and CGB HDMA/GPDMA (0xFF51-0xFF55). It owns
/// the registers and the transfer state, but the byte movement itself is driven
/// by the motherboard, since reading the source is a bus-master operation and
/// writing OAM/VRAM uses the PPU's direct DMA ports. The engine only decides
/// *what* to move and *when*.
pub struct Dma {
    mode: GBMode,

    // OAM DMA
    oam_src: u16,
    pub oam_progress: u16,
    pub oam_active: bool,
    oam_starting: bool,
    pub oam_latch: u8,

    // HDMA (CGB). hdma_len == 0xFF means inactive.
    hdma_src: u16,
    hdma_dst: u16,
    hdma_len: u8,
    hdma_hblank: bool,
    gpdma_request: bool,
}

impl Dma {
    pub fn new(mode: GBMode) -> Self {
        Self {
            mode,
            oam_src: 0,
            oam_progress: 0,
            oam_active: false,
            oam_starting: false,
            oam_latch: 0xFF,
            hdma_src: 0,
            hdma_dst: 0x8000,
            hdma_len: 0xFF,
            hdma_hblank: false,
            gpdma_request: false,
        }
    }

    fn owns(addr: u16) -> bool {
        matches!(addr, 0xFF46 | 0xFF51..=0xFF55)
    }

    fn start_oam(&mut self, value: u8) {
        self.oam_src = (value as u16) << 8;
        self.oam_progress = 0;
        self.oam_active = true;
        self.oam_starting = true;
    }

    fn start_hdma(&mut self, v: u8) {
        // Clearing bit 7 while an HBlank transfer is active cancels it.
        if self.hdma_len != 0xFF && (v & 0x80) == 0 {
            self.hdma_len = 0xFF;
            self.hdma_hblank = false;
            return;
        }
        self.hdma_len = v & 0x7F;
        self.hdma_hblank = (v & 0x80) != 0;
        if !self.hdma_hblank {
            // General-purpose DMA: the motherboard performs the whole copy at
            // once, halting the CPU.
            self.gpdma_request = true;
        }
    }

    // -- motherboard-facing transfer control ------------------------------

    /// Next OAM-DMA source byte address to read this M-cycle, or None. Consumes
    /// the one-M-cycle startup delay.
    pub fn oam_next(&mut self) -> Option<u16> {
        if !self.oam_active {
            return None;
        }
        if self.oam_starting {
            self.oam_starting = false;
            return None;
        }
        Some(self.oam_src + self.oam_progress)
    }

    /// Record a byte fetched for OAM DMA; advance and deactivate when done.
    pub fn oam_feed(&mut self, byte: u8) {
        self.oam_latch = byte;
        self.oam_progress += 1;
        if self.oam_progress >= 0xA0 {
            self.oam_active = false;
        }
    }

    /// DMG OAM-DMA bus conflict: a CPU read of the same external bus region as
    /// the active DMA source returns the DMA's latched byte.
    pub fn oam_conflict(&self, addr: u16) -> bool {
        if self.mode != GBMode::DMG || !self.oam_active {
            return false;
        }
        let bus = |x: u16| match x {
            0x8000..=0x9FFF => 1u8,
            0x0000..=0x7FFF | 0xA000..=0xFDFF => 0,
            _ => 2,
        };
        let b = bus(addr);
        b != 2 && b == bus(self.oam_src)
    }

    pub fn take_gpdma(&mut self) -> bool {
        std::mem::take(&mut self.gpdma_request)
    }

    pub fn hdma_hblank_active(&self) -> bool {
        self.hdma_len != 0xFF && self.hdma_hblank
    }

    /// Advance one 0x10-byte HDMA block: returns the (src, dst) pairs to copy,
    /// then updates the running length. The motherboard performs the reads and
    /// VRAM writes.
    pub fn hdma_block(&mut self) -> [(u16, u16); 0x10] {
        let mut pairs = [(0u16, 0u16); 0x10];
        for i in 0..0x10u16 {
            pairs[i as usize] = (self.hdma_src.wrapping_add(i), self.hdma_dst.wrapping_add(i));
        }
        self.hdma_src = self.hdma_src.wrapping_add(0x10);
        self.hdma_dst = self.hdma_dst.wrapping_add(0x10);
        if self.hdma_len != 0 {
            self.hdma_len -= 1;
        } else {
            self.hdma_len = 0xFF;
        }
        pairs
    }

    pub fn hdma_len(&self) -> u8 {
        self.hdma_len
    }
}

impl Chip for Dma {
    fn bus(&mut self, pins: &mut Pins) -> Ticked {
        if pins.selected(Self::owns(pins.address)) {
            let cgb = self.mode != GBMode::DMG;
            match (pins.address, pins.dir) {
                (0xFF46, BusDir::Read) => pins.data = (self.oam_src >> 8) as u8,
                (0xFF46, BusDir::Write) => self.start_oam(pins.data),
                (0xFF51, BusDir::Write) if cgb => {
                    self.hdma_src = (self.hdma_src & 0x00FF) | ((pins.data as u16) << 8)
                }
                (0xFF52, BusDir::Write) if cgb => {
                    self.hdma_src = (self.hdma_src & 0xFF00) | (pins.data as u16 & 0xF0)
                }
                (0xFF53, BusDir::Write) if cgb => {
                    self.hdma_dst =
                        0x8000 | (self.hdma_dst & 0x00FF) | ((pins.data as u16 & 0x1F) << 8)
                }
                (0xFF54, BusDir::Write) if cgb => {
                    self.hdma_dst = (self.hdma_dst & 0xFF00) | (pins.data as u16 & 0xF0)
                }
                (0xFF55, BusDir::Read) if cgb => pins.data = self.hdma_len,
                (0xFF55, BusDir::Write) if cgb => self.start_hdma(pins.data),
                (0xFF51..=0xFF55, BusDir::Read) => pins.data = 0xFF, // DMG: inert
                _ => {}
            }
        }
        Ticked::default()
    }
}
