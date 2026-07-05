use crate::components::memory::Memory;
use super::bus::{BusDir, Chip, Pins, Ticked};
use super::interrupt::Interrupts;
use crate::components::ppu::ppu::PPU as CorePpu;
use crate::config::Config;
use crate::framebuffer::FramebufferWriter;

/// The PPU as a peer chip. It owns the existing dot-accurate line renderer and
/// adapts it to the bus. The renderer is boxed: it embeds ~384 KB of color
/// lookup tables, and keeping it on the heap stops that from being copied
/// through every stack frame during construction (which overflows the thread
/// stack in debug builds, where there is no return-value optimization).
pub struct Ppu {
    core: Box<CorePpu>,
}

impl Ppu {
    pub fn new(config: Config, framebuffer: FramebufferWriter, rom_is_cgb: bool) -> Self {
        Self {
            core: Box::new(CorePpu::new(config, framebuffer, rom_is_cgb)),
        }
    }

    /// Addresses the PPU is authoritative for. Deliberately excludes 0xFF46
    /// (OAM DMA) and 0xFF4C/0xFF4D so the DMA and speed-switch owners can claim
    /// them without a bus conflict.
    fn owns(addr: u16) -> bool {
        matches!(addr,
            0x8000..=0x9FFF
            | 0xFE00..=0xFE9F
            | 0xFF40..=0xFF45
            | 0xFF47..=0xFF4B
            | 0xFF4F
            | 0xFF68..=0xFF6C)
    }

    /// OAM write port for the OAM-DMA engine, which drives OAM while holding
    /// the bus as master (unconditional, bypassing CPU mode-gating).
    pub fn write_oam(&mut self, index: u16, value: u8) {
        self.core.write_oam(index, value);
    }

    /// VRAM write port for the HDMA/GPDMA engine (unconditional).
    pub fn write_vram_dma(&mut self, addr: u16, value: u8) {
        self.core.write_vram_direct(addr, value);
    }

    /// Forwarded from a write to 0xFF50: mirrors the legacy hand-off (clears
    /// VRAM and drops the PPU's boot-rom palette behaviour).
    pub fn on_boot_rom_disabled(&mut self) {
        self.core.clear_vram();
        self.core.disable_boot_rom();
    }
}

impl Chip for Ppu {
    fn advance(&mut self, base_dot: bool) -> Ticked {
        // Internal advance: one dot per base-clock tick.
        if base_dot {
            self.core.cycle(1);
        }
        let (irq_bits, hblank_edge) = self.core.take_events();
        Ticked {
            irq: Interrupts::from_bits_truncate(irq_bits),
            hblank_edge,
        }
    }

    fn bus(&mut self, pins: &mut Pins) -> Ticked {
        if pins.transfer && Self::owns(pins.address) {
            match pins.dir {
                BusDir::Read => pins.data = self.core.read(pins.address),
                BusDir::Write => self.core.write(pins.address, pins.data),
                BusDir::Idle => {}
            }
        }
        Ticked::default()
    }
}
