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
            apu: Apu::new(config.clone()),
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

    /// Read a byte of CPU-addressable memory without side effects (cartridge,
    /// WRAM, HRAM, and the sysbus-owned registers). For inspection/testing.
    pub fn peek(&self, a: u16) -> u8 {
        self.sysbus.peek(a)
    }

    /// Bytes the program has transmitted over the serial port.
    pub fn serial_output(&self) -> &[u8] {
        self.sysbus.serial_output()
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
        let was_halted = self.cpu.is_halted();
        self.cpu.run_free_acts();

        // A 16-bit inc/dec through the OAM region this M-cycle glitches OAM on
        // the DMG when the PPU is scanning it; the PPU gates on mode itself.
        if self.cpu.take_oam_glitch() {
            self.ppu.corrupt_oam_inc();
        }

        if self.cpu.is_halted() {
            if self.ic.pending() != Interrupts::empty() {
                // Executing HALT with an interrupt already pending but IME
                // clear does not halt; it arms the HALT bug so the next fetch
                // reads the following byte twice. A wake from an earlier HALT
                // (was_halted) is the ordinary resume path.
                if !was_halted && !self.cpu.ime() {
                    self.cpu.trigger_halt_bug();
                }
                self.cpu.wake();
            } else {
                self.step_oam_dma();
                self.idle_mcycle();
                return false;
            }
        }

        // OAM DMA moves one byte per M-cycle, concurrent with the CPU.
        self.step_oam_dma();

        self.cpu.setup(&mut self.pins);
        self.run_dots();

        // DMG OAM-DMA bus conflict: a CPU read of the DMA's bus region returns
        // the DMA latch instead of the addressed byte.
        if self.pins.dir == BusDir::Read && self.dma.oam_conflict(self.pins.address) {
            self.pins.data = self.dma.oam_latch;
        }

        let fetched = self.cpu.complete(&self.pins);
        self.pins.transfer = false;

        // Interrupt servicing is decided at fetch time: a request that rose at
        // any dot of this fetch M-cycle (the PPU/timer requests merged during
        // run_dots above) converts the just-fetched opcode into the ISR's
        // first internal cycle. Sub-M-cycle sampling within the fetch is not
        // modelled; mooneye's intr tests would pin that edge.
        if fetched {
            let pending = self.ic.pending();
            if let Some(bit) = self.cpu.offer_interrupt(pending) {
                self.ic.acknowledge(bit);
            }
        }
        fetched
    }

    /// Advance the four T-cycles of an M-cycle. Chips advance their internal
    /// clock every dot (base-domain gated by the divider). The bus transfer
    /// resolves on the last dot *before* the chips advance through it: the bus
    /// settles during the M-cycle's final T-cycle, so the PPU pixel clocked on
    /// that dot (and the timer/APU state stepped on it) already observes the
    /// written value. This is what makes a mid-Mode-3 palette write's
    /// transitional value land on the hardware-correct pixel.
    fn run_dots(&mut self) {
        for dot in 0..4u8 {
            self.pins.transfer = dot == 3;
            let base_dot = self.clock.tick(self.sysbus.double_speed());

            let mut ticked = Ticked::default();
            if self.pins.transfer {
                ticked.merge(self.timer.bus(&mut self.pins));
                ticked.merge(self.ic.bus(&mut self.pins));
                ticked.merge(self.ppu.bus(&mut self.pins));
                ticked.merge(self.dma.bus(&mut self.pins));
                self.apu.bus(&mut self.pins);
                ticked.merge(self.sysbus.bus(&mut self.pins));
            }

            ticked.merge(self.timer.advance(base_dot));
            ticked.merge(self.ppu.advance(base_dot));
            ticked.merge(self.sysbus.advance(base_dot));

            // The APU is a base-clock device: advance its frame sequencer and
            // channel frequency timers once per base dot.
            if base_dot {
                self.apu.advance(self.timer.div(), self.sysbus.double_speed());
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
