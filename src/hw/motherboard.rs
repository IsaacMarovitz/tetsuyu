use super::apu::Apu;
use super::bus::{BusDir, BusMaster, Chip, Pins, Ticked};
use super::clock::Clock;
use super::cpu::Cpu;
use super::dma::Dma;
use super::interrupt::{InterruptController, Interrupts};
use super::ppu::Ppu;
use super::sysbus::SystemBus;
use super::timer::Timer;
use crate::components::joypad::JoypadButton;
use crate::config::Config;
use crate::framebuffer::FramebufferWriter;
use crate::mbc::header::Header;

/// The mainboard: owns the chips, the clock, and the traces. Its logic is
/// arbitration, routing, and driving the DMA — it holds no peripheral internals
/// and no address decode (each chip decodes itself).
pub struct Motherboard {
    cpu: Cpu,
    clock: Clock,
    pins: Pins,
    ic: InterruptController,
    timer: Timer,
    ppu: Ppu,
    apu: Apu,
    dma: Dma,
    sysbus: SystemBus,
}

impl Motherboard {
    pub fn new(
        rom: Vec<u8>,
        header: Header,
        config: Config,
        boot_rom: [u8; 0x900],
        framebuffer: FramebufferWriter,
        rom_is_cgb: bool,
    ) -> Self {
        let mode = config.mode;
        Self {
            cpu: Cpu::new(mode),
            clock: Clock::new(),
            pins: Pins::new(),
            ic: InterruptController::new(),
            timer: Timer::new(),
            ppu: Ppu::new(config.clone(), framebuffer, rom_is_cgb),
            apu: Apu::new(config.apu_config, config.mode),
            dma: Dma::new(mode),
            sysbus: SystemBus::new(rom, header, &config, boot_rom),
        }
    }

    /// Convenience constructor matching the legacy `CPU::new`: loads the boot
    /// ROM from the config paths and derives the CGB flag from the header.
    pub fn from_config(
        rom: Vec<u8>,
        header: Header,
        config: Config,
        framebuffer: FramebufferWriter,
    ) -> Self {
        use crate::components::mode::GBMode;
        use crate::mbc::header::CGBFlag;
        use std::fs::File;
        use std::io::Read;

        let boot_file = match config.mode {
            GBMode::DMG => config.dmg_boot_rom.clone(),
            GBMode::CGB => config.cgb_boot_rom.clone(),
        };

        let mut boot_rom = [0u8; 0x900];
        let mut boot_rom_vec = Vec::new();
        let mut boot = match File::open(boot_file.clone()) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Failed to open Boot ROM at \"{}\": {}", boot_file, err);
                std::process::exit(1);
            }
        };
        boot.read_to_end(&mut boot_rom_vec)
            .expect("Failed to read Boot ROM!");

        if config.mode == GBMode::DMG {
            boot_rom[0..=0x00FF].copy_from_slice(boot_rom_vec.as_slice());
        } else {
            boot_rom[0..=0x08FF].copy_from_slice(boot_rom_vec.as_slice());
        }

        let rom_is_cgb = matches!(
            header.cgb_flag,
            CGBFlag::CGBOnly | CGBFlag::BackwardsCompatible
        );

        Self::new(rom, header, config, boot_rom, framebuffer, rom_is_cgb)
    }

    pub fn joypad_down(&mut self, b: JoypadButton) {
        self.sysbus.joypad_down(b);
    }

    pub fn joypad_up(&mut self, b: JoypadButton) {
        self.sysbus.joypad_up(b);
    }

    /// Run one instruction; returns the elapsed T-cycles (4 per M-cycle).
    pub fn step(&mut self) -> u32 {
        let mut mcycles = 0u32;
        loop {
            let fetched = self.m_cycle();
            mcycles += 1;
            if fetched {
                break;
            }
        }
        // A general-purpose HDMA requested this instruction copies at once,
        // halting the CPU; run it now.
        if self.dma.take_gpdma() {
            self.run_gpdma();
        }
        // STOP requests a speed switch if KEY1 is armed.
        if self.cpu.take_speed_switch() {
            self.sysbus.try_speed_switch();
        }
        mcycles * 4
    }

    // -- one M-cycle -------------------------------------------------------

    fn m_cycle(&mut self) -> bool {
        self.cpu.run_free_acts();

        if self.cpu.is_halted() {
            if self.ic.pending() != Interrupts::empty() {
                self.cpu.wake();
            } else {
                self.step_oam_dma();
                self.idle_mcycle();
                return false;
            }
        }

        // OAM DMA moves one byte per M-cycle, concurrent with the CPU.
        self.step_oam_dma();

        if self.cpu.at_instruction_boundary() {
            let pending = self.ic.pending();
            if let Some(bit) = self.cpu.offer_interrupt(pending) {
                self.ic.acknowledge(bit);
            }
        }

        self.cpu.setup(&mut self.pins);
        self.run_dots();

        // DMG OAM-DMA bus conflict: a CPU read of the DMA's bus region returns
        // the DMA latch instead of the addressed byte.
        if self.pins.dir == BusDir::Read && self.dma.oam_conflict(self.pins.address) {
            self.pins.data = self.dma.oam_latch;
        }

        let fetched = self.cpu.complete(&self.pins);
        self.pins.transfer = false;
        fetched
    }

    /// Advance the four T-cycles of an M-cycle. Chips advance their internal
    /// clock every dot (base-domain gated by the divider); the bus transfer
    /// resolves on the last dot.
    fn run_dots(&mut self) {
        for dot in 0..4u8 {
            self.pins.transfer = dot == 3;
            let base_dot = self.clock.tick(self.sysbus.double_speed());

            let mut ticked = Ticked::default();
            ticked.merge(self.timer.advance(base_dot));
            ticked.merge(self.ppu.advance(base_dot));
            ticked.merge(self.sysbus.advance(base_dot));

            // The APU is a base-clock device: advance its frame sequencer and
            // channel frequency timers once per base dot, before the transfer
            // resolves so a wave-RAM read observes the current sample position.
            if base_dot {
                self.apu.advance(self.timer.div(), self.sysbus.double_speed());
            }

            if self.pins.transfer {
                ticked.merge(self.timer.bus(&mut self.pins));
                ticked.merge(self.ic.bus(&mut self.pins));
                ticked.merge(self.ppu.bus(&mut self.pins));
                ticked.merge(self.dma.bus(&mut self.pins));
                self.apu.bus(&mut self.pins);
                ticked.merge(self.sysbus.bus(&mut self.pins));
            }

            self.ic.request(ticked.irq);
            if ticked.hblank_edge {
                self.step_hdma_hblank();
            }
        }

        // A write to 0xFF50 disables the boot ROM; forward to the PPU.
        if self.sysbus.take_boot_disabled() {
            self.ppu.on_boot_rom_disabled();
        }
    }

    fn idle_mcycle(&mut self) {
        self.pins.dir = BusDir::Idle;
        self.run_dots();
        self.pins.transfer = false;
    }

    // -- DMA driving -------------------------------------------------------

    /// Read an address as a non-CPU bus master, without advancing any chip's
    /// internal clock. Used by the DMA engine for its source fetches.
    fn bus_read(&mut self, addr: u16, master: BusMaster) -> u8 {
        self.pins.address = addr;
        self.pins.dir = BusDir::Read;
        self.pins.master = master;
        self.pins.transfer = true;
        self.pins.data = 0xFF;
        self.ppu.bus(&mut self.pins);
        self.dma.bus(&mut self.pins);
        self.sysbus.bus(&mut self.pins);
        self.pins.transfer = false;
        self.pins.master = BusMaster::Cpu;
        self.pins.data
    }

    fn step_oam_dma(&mut self) {
        if let Some(src) = self.dma.oam_next() {
            let byte = self.bus_read(src, BusMaster::OamDma);
            let offset = self.dma.oam_progress;
            self.ppu.write_oam(offset, byte);
            self.dma.oam_feed(byte);
        }
    }

    fn step_hdma_hblank(&mut self) {
        if !self.dma.hdma_hblank_active() {
            return;
        }
        self.hdma_copy_block();
    }

    fn run_gpdma(&mut self) {
        let blocks = self.dma.hdma_len() as u16 + 1;
        for _ in 0..blocks {
            self.hdma_copy_block();
            // The bus still advances 8 M-cycles per 16-byte block while the CPU
            // is halted (doubled in double-speed).
            let idles = 8u32 << self.sysbus.double_speed() as u32;
            for _ in 0..idles {
                self.idle_mcycle();
            }
        }
    }

    fn hdma_copy_block(&mut self) {
        let pairs = self.dma.hdma_block();
        for (src, dst) in pairs {
            let byte = self.bus_read(src, BusMaster::OamDma);
            self.ppu.write_vram_dma(dst, byte);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::mode::GBMode;

    // A minimal ROM-only cartridge holding a test program at 0x0000, with the
    // boot ROM disabled so execution starts in cart space.
    fn boot_into(program: &[u8]) -> Motherboard {
        let mut rom = vec![0u8; 0x8000];
        rom[..program.len()].copy_from_slice(program);
        rom[0x0147] = 0x00; // ROM only
        let header = Header::new(rom.clone());
        let (fb, _reader) = crate::framebuffer::create_framebuffer_pair();
        let mut config = Config::default();
        config.mode = GBMode::DMG;
        let mut mb = Motherboard::new(rom, header, config, [0u8; 0x900], fb, false);
        mb.sysbus.disable_boot();
        mb
    }

    #[test]
    fn runs_a_few_instructions() {
        // LD A,$12 ; LD ($C000),A ; JP $0000
        let mut mb = boot_into(&[0x3E, 0x12, 0xEA, 0x00, 0xC0, 0xC3, 0x00, 0x00]);
        for _ in 0..8 {
            mb.step();
        }
        assert_eq!(mb.sysbus.peek(0xC000), 0x12);
    }
}
