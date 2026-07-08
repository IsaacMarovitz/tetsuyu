use super::bus::{BusDir, Chip, Pins, Ticked};
use crate::components::mode::GBMode;

pub struct Dma {
    mode: GBMode,

    // OAM DMA
    oam_src: u16,
    pub oam_progress: u16,
    pub oam_active: bool,
    oam_setup: u8,
    oam_pending_src: u16,
    oam_running: bool,

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
            oam_setup: 0,
            oam_pending_src: 0,
            oam_running: false,
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
        self.oam_pending_src = (value as u16) << 8;
        self.oam_setup = 2;
        self.oam_active = true;
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

    pub fn oam_next(&mut self) -> Option<u16> {
        if !self.oam_active {
            return None;
        }

        if self.oam_setup > 0 {
            self.oam_setup -= 1;
            if self.oam_setup == 0 {
                // Setup elapsed: the pending source takes over and the transfer
                // begins moving its first byte this very M-cycle.
                self.oam_src = self.oam_pending_src;
                self.oam_progress = 0;
                self.oam_running = true;
                return Some(self.oam_src + self.oam_progress);
            }
            // Still in setup. A restart's previous transfer keeps running;
            // a fresh start moves nothing yet.
            if self.oam_running {
                if self.oam_progress >= 0xA0 {
                    self.oam_running = false;
                    return None;
                }
                return Some(self.oam_src + self.oam_progress);
            }
            return None;
        }

        if self.oam_progress >= 0xA0 {
            // The 160th byte moved on the previous M-cycle; the conflict held
            // through it, and only now does the engine go idle.
            self.oam_active = false;
            self.oam_running = false;
            return None;
        }
        Some(self.oam_src + self.oam_progress)
    }

    /// Record a byte fetched for OAM DMA and advance. Deactivation is deferred
    /// to the next `oam_next` so the final transfer M-cycle still conflicts.
    pub fn oam_feed(&mut self, _byte: u8) {
        self.oam_progress += 1;
    }

    pub fn oam_conflict(&self, addr: u16) -> bool {
        if !self.oam_running {
            return false;
        }

        // OAM is inaccessible on both models while it is being written.
        if matches!(addr, 0xFE00..=0xFE9F) {
            return true;
        }

        match self.mode {
            GBMode::DMG => false,
            _ => Self::cgb_same_bus(self.oam_src, addr),
        }
    }

    fn cgb_same_bus(src: u16, addr: u16) -> bool {
        let external = |a: u16| matches!(a, 0x0000..=0x7FFF | 0xA000..=0xBFFF);
        let internal = |a: u16| matches!(a, 0xC000..=0xFDFF);
        (external(src) && external(addr)) || (internal(src) && internal(addr))
    }

    pub fn take_gpdma(&mut self) -> bool {
        std::mem::take(&mut self.gpdma_request)
    }

    pub fn hdma_hblank_active(&self) -> bool {
        self.hdma_len != 0xFF && self.hdma_hblank
    }

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
