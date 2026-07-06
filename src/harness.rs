//! Headless test harness.
//!
//! Drives a [`Motherboard`] with no window, audio, or real-time pacing, runs it
//! until a stop condition is met (a register reaches a value, a frame count, or
//! a cycle budget), then exposes serial output, memory, and the framebuffer for
//! validation. Construction returns a `Result` instead of exiting the process,
//! so a failed load fails one test rather than the whole runner.

use crate::components::mode::GBMode;
use crate::config::Config;
use crate::framebuffer::{create_framebuffer_pair, FramebufferReader};
use crate::hw::motherboard::Motherboard;
use crate::mbc::header::{CGBFlag, Header};
use std::fs;

/// Condition that ends a [`Harness::run_until`] run.
#[derive(Debug, Clone, Copy)]
pub enum StopCondition {
    /// A read of `addr` (CPU-addressable memory / sysbus register) equals
    /// `value`. Polled after each instruction.
    RegisterEquals { addr: u16, value: u8 },
    /// At least this many frames (VBlanks) have been produced.
    Frames(u64),
    /// At least this many T-cycles have elapsed.
    Cycles(u64),
}

/// Why a run ended.
#[derive(Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// The stop condition was met.
    Met,
    /// The cycle budget elapsed before the stop condition was met.
    TimedOut,
}

pub struct Harness {
    mb: Motherboard,
    fb: FramebufferReader,
    cycles: u64,
}

impl Harness {
    /// Load `rom_path` and build a headless machine using `config` (including
    /// its boot-rom paths and mode).
    pub fn new(rom_path: &str, config: Config) -> Result<Self, String> {
        let rom = fs::read(rom_path).map_err(|e| format!("open ROM \"{rom_path}\": {e}"))?;
        let header = Header::new(rom.clone());
        let rom_is_cgb = matches!(
            header.cgb_flag,
            CGBFlag::CGBOnly | CGBFlag::BackwardsCompatible
        );

        let boot_path = match config.mode {
            GBMode::DMG => config.dmg_boot_rom.clone(),
            GBMode::CGB => config.cgb_boot_rom.clone(),
        };
        let boot_vec =
            fs::read(&boot_path).map_err(|e| format!("open boot ROM \"{boot_path}\": {e}"))?;
        let mut boot_rom = [0u8; 0x900];
        let end = boot_vec.len().min(boot_rom.len());
        boot_rom[..end].copy_from_slice(&boot_vec[..end]);

        let (writer, fb) = create_framebuffer_pair();
        let mb = Motherboard::new(rom, header, config, boot_rom, writer, rom_is_cgb);

        Ok(Self { mb, fb, cycles: 0 })
    }

    /// Step one instruction at a time until `stop` is met or `max_cycles`
    /// T-cycles have elapsed, whichever comes first.
    pub fn run_until(&mut self, stop: StopCondition, max_cycles: u64) -> RunOutcome {
        loop {
            self.cycles += self.mb.step() as u64;
            let frames = self.fb.poll();

            let met = match stop {
                StopCondition::RegisterEquals { addr, value } => self.mb.peek(addr) == value,
                StopCondition::Frames(n) => frames >= n,
                StopCondition::Cycles(n) => self.cycles >= n,
            };
            if met {
                return RunOutcome::Met;
            }
            if self.cycles >= max_cycles {
                return RunOutcome::TimedOut;
            }
        }
    }

    /// T-cycles elapsed since construction.
    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Read one byte of CPU-addressable memory (no side effects).
    pub fn peek(&self, addr: u16) -> u8 {
        self.mb.peek(addr)
    }

    /// Read `len` consecutive bytes starting at `start`.
    pub fn read_range(&self, start: u16, len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| self.mb.peek(start.wrapping_add(i as u16)))
            .collect()
    }

    /// Bytes transmitted over the serial port so far.
    pub fn serial(&self) -> &[u8] {
        self.mb.serial_output()
    }

    /// The most recent complete frame as RGBA (`4 * 160 * 144` bytes).
    pub fn framebuffer(&mut self) -> &[u8] {
        self.fb.get_latest_frame()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The project's real config (has the local boot-rom paths). Tests that need
    /// a boot ROM early-return when it is absent rather than failing.
    fn project_config() -> Option<Config> {
        let s = fs::read_to_string("./config.toml").ok()?;
        let mut config: Config = toml::from_str(&s).ok()?;
        config.apu_config.master_enabled = false;

        Some(config)
    }

    const FRAME_CYCLES: u64 = 70_224;

    #[test]
    fn m3_bgp_change_renders_frames() {
        let Some(config) = project_config() else {
            eprintln!("skipping: no ./config.toml with boot-rom paths");
            return;
        };

        let mut h = match Harness::new("roms/mealybug/m3_bgp_change.gb", config) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("skipping: {e}");
                return;
            }
        };

        let outcome = h.run_until(StopCondition::Frames(200), 250 * FRAME_CYCLES);
        assert_eq!(outcome, RunOutcome::Met, "did not reach 200 frames in budget");

        // Past the boot logo the test draws multiple BGP shades; a blank frame
        // would be a single color.
        let mut colors = HashSet::new();
        for px in h.framebuffer().chunks_exact(4) {
            colors.insert([px[0], px[1], px[2]]);
        }
        assert!(
            colors.len() >= 3,
            "expected several shades, got {}",
            colors.len()
        );
    }
}
