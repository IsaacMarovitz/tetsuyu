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
    /// M-cycles remaining before a freshly-written OAM DMA begins moving bytes
    /// and asserting the bus conflict. Hardware needs two M-cycles of setup
    /// after the $FF46 write: the write M-cycle itself and one more still have
    /// OAM accessible; the transfer (and conflict) begin on the *third*
    /// M-cycle. `oam_dma_start` pins this exactly. During these cycles a
    /// previously-running transfer (a restart) keeps moving bytes and
    /// conflicting until the new one takes over.
    oam_setup: u8,
    /// Source high byte latched by the pending $FF46 write, applied when
    /// `oam_setup` elapses. Distinct from `oam_src` so a restart's in-flight
    /// transfer keeps its own source during the 2-cycle setup.
    oam_pending_src: u16,
    /// True while a transfer is actively moving bytes (and therefore asserting
    /// the DMG bus conflict). False during the 2-cycle setup of a *fresh* DMA,
    /// but a restart leaves the previous transfer running through setup.
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
        // Latch the new source and arm the 2-M-cycle setup. If a transfer is
        // already running (a restart), it keeps moving bytes and asserting the
        // conflict until the setup elapses; only then does the new source take
        // over with progress reset. For a fresh start the engine is simply
        // marked active with no transfer/conflict until setup completes.
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

    // -- motherboard-facing transfer control ------------------------------

    /// Next OAM-DMA source byte address to read this M-cycle, or None. Drives
    /// the 2-M-cycle setup delay: a freshly written DMA moves no bytes (and
    /// asserts no conflict) for two M-cycles after the $FF46 write, then begins
    /// transferring on the third. A restart written over a running transfer
    /// lets the old transfer keep moving bytes through those two cycles until
    /// the new source takes over. Retirement is deferred to the M-cycle *after*
    /// the 160th byte moved, so the conflict still holds on that final transfer
    /// cycle (the acceptance tests align a read to exactly it).
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

    /// DMG OAM-DMA bus conflict: while a transfer is actively moving bytes, a
    /// CPU read of OAM ($FE00-$FE9F) is driven by the DMA unit instead and comes
    /// back as $FF. This is the conflict the mooneye acceptance suite exercises:
    /// `oam_dma_timing`/`oam_dma_start`/`oam_dma_restart` read OAM during a
    /// transfer, and the instruction-timing tests execute from `OAM-1` so the
    /// instruction's *operand* (at $FE00, in OAM) conflicts while its opcode
    /// (fetched from $FDFF, in echo RAM) is read normally. Gated on
    /// `oam_running`, so it does not assert during the 2-M-cycle setup of a
    /// fresh DMA (OAM still accessible then).
    ///
    /// (Pandocs describes a broader DMG rule — during OAM DMA the CPU can reach
    /// only HRAM — but modelling it that broadly conflicts the execute-from-echo
    /// opcode fetch the timing tests rely on. The OAM-region conflict is what
    /// the suite actually pins; the wider external-bus behaviour is left for a
    /// later, finer model. CGB is unaffected: it never conflicts here.)
    pub fn oam_conflict(&self, addr: u16) -> bool {
        if self.mode != GBMode::DMG || !self.oam_running {
            return false;
        }
        matches!(addr, 0xFE00..=0xFE9F)
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
