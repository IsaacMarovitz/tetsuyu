//! Headless test harness.
//!
//! Drives a [`Motherboard`] with no window, audio, or real-time pacing, runs it
//! until a stop condition is met (a register reaches a value, a frame count, or
//! a cycle budget), then exposes serial output, memory, and the framebuffer for
//! validation. Construction returns a `Result` instead of exiting the process,
//! so a failed load fails one test rather than the whole runner.

use std::cmp::PartialEq;
use crate::components::mode::GBMode;
use crate::components::ppu::ppu::{SCREEN_H, SCREEN_W};
use crate::config::Config;
use crate::framebuffer::{FramebufferReader, create_framebuffer_pair};
use crate::hw::motherboard::Motherboard;
use crate::mbc::header::{CGBFlag, Header};
use std::fs;
use std::path::Path;
use crate::components::prelude::Registers;

/// Condition that ends a [`Harness::run_until`] run.
#[derive(Debug, Clone, Copy)]
pub enum StopCondition {
    /// A read of `addr` (CPU-addressable memory / sysbus register) equals
    /// `value`. Polled after each instruction.
    RegisterEquals { addr: u16, value: u8 },
    RegistersEqual(Registers),
    /// The CPU executed `LD B,B` (0x40), the test-ROM magic breakpoint.
    MagicBreak,
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
                StopCondition::RegistersEqual(r) => self.cpu_regs() == r,
                StopCondition::MagicBreak => self.mb.magic_break(),
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

    /// CPU registers.
    pub fn cpu_regs(&self) -> Registers {
        self.mb.cpu_regs()
    }

    /// True once the CPU has hit the `LD B,B` magic breakpoint.
    pub fn magic_break(&self) -> bool {
        self.mb.magic_break()
    }

    /// The most recent complete frame as RGBA (`4 * 160 * 144` bytes).
    pub fn framebuffer(&mut self) -> &[u8] {
        self.fb.get_latest_frame()
    }
}

/// A decoded reference image, expanded to 8-bit RGBA.
pub struct RefImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl RefImage {
    /// Decode the PNG at `path` into 8-bit RGBA. Supports the 8-bit grayscale,
    /// RGB, RGBA, and palette-indexed encodings the mealybug / acid2 references
    /// ship as.
    pub fn load_png(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let file = fs::File::open(path)
            .map_err(|e| format!("open reference \"{}\": {e}", path.display()))?;
        // The mealybug references are 1-/2-bit indexed or grayscale PNGs; expand
        // paletted + low-bit-depth pixels to straight 8-bit channels (and strip
        // any 16-bit down to 8) so the byte handling below is uniform.
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("decode \"{}\": {e}", path.display()))?;

        let buf_size = reader
            .output_buffer_size()
            .ok_or_else(|| format!("\"{}\": image too large", path.display()))?;
        let mut raw = vec![0u8; buf_size];
        let info = reader
            .next_frame(&mut raw)
            .map_err(|e| format!("read \"{}\": {e}", path.display()))?;
        raw.truncate(info.buffer_size());

        if info.bit_depth != png::BitDepth::Eight {
            return Err(format!(
                "\"{}\": unsupported bit depth {:?} (expected 8-bit)",
                path.display(),
                info.bit_depth
            ));
        }

        let (w, h) = (info.width as usize, info.height as usize);
        let px = w * h;
        let mut rgba = vec![0u8; px * 4];

        match info.color_type {
            png::ColorType::Grayscale => {
                for (i, &g) in raw.iter().take(px).enumerate() {
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, 0xFF]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for i in 0..px {
                    let g = raw[i * 2];
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, raw[i * 2 + 1]]);
                }
            }
            png::ColorType::Rgb => {
                for i in 0..px {
                    rgba[i * 4..i * 4 + 3].copy_from_slice(&raw[i * 3..i * 3 + 3]);
                    rgba[i * 4 + 3] = 0xFF;
                }
            }
            png::ColorType::Rgba => rgba.copy_from_slice(&raw[..px * 4]),
            png::ColorType::Indexed => {
                let palette =
                    reader.info().palette.as_ref().ok_or_else(|| {
                        format!("\"{}\": indexed PNG has no palette", path.display())
                    })?;
                for i in 0..px {
                    let idx = raw[i] as usize;
                    rgba[i * 4] = palette.get(idx * 3).copied().unwrap_or(0);
                    rgba[i * 4 + 1] = palette.get(idx * 3 + 1).copied().unwrap_or(0);
                    rgba[i * 4 + 2] = palette.get(idx * 3 + 2).copied().unwrap_or(0);
                    rgba[i * 4 + 3] = 0xFF;
                }
            }
        }

        Ok(Self {
            width: w,
            height: h,
            rgba,
        })
    }
}

/// Result of comparing a rendered frame to a reference image.
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub total: usize,
    pub matched: usize,
    /// First (x, y) that differs, scanning row-major; `None` when exact.
    pub first_diff: Option<(usize, usize)>,
}

impl DiffReport {
    pub fn match_pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.matched as f64 * 100.0 / self.total as f64
        }
    }

    pub fn is_exact(&self) -> bool {
        self.total > 0 && self.first_diff.is_none()
    }
}

/// Compare a rendered RGBA `frame` (160×144) to `reference` by direct RGB
/// equality. The mealybug references encode the four DMG shades as the
/// canonical greyscale values $00/$55/$AA/$FF; run the emulator with the
/// matching palette (see `mealybug_palette`) and the comparison is exact
/// byte equality — no canonicalization, no cross-image coupling.
pub fn compare_frame(frame: &[u8], reference: &RefImage) -> Result<DiffReport, String> {
    if reference.width != SCREEN_W || reference.height != SCREEN_H {
        return Err(format!(
            "reference is {}×{}, expected {SCREEN_W}×{SCREEN_H}",
            reference.width, reference.height
        ));
    }
    if frame.len() < SCREEN_W * SCREEN_H * 4 {
        return Err(format!("frame is {} bytes, too small", frame.len()));
    }

    let mut matched = 0usize;
    let mut first_diff = None;
    for y in 0..SCREEN_H {
        for x in 0..SCREEN_W {
            let i = (y * SCREEN_W + x) * 4;
            if frame[i..i + 3] == reference.rgba[i..i + 3] {
                matched += 1;
            } else if first_diff.is_none() {
                first_diff = Some((x, y));
            }
        }
    }

    Ok(DiffReport {
        total: SCREEN_W * SCREEN_H,
        matched,
        first_diff,
    })
}

/// Run `rom_path` for `frames` VBlanks under `config`, then compare the final
/// framebuffer to the PNG at `expected_png`. The one call every bring-up step
/// uses; pass a `config` whose `ppu_config.use_fifo_renderer` selects the path
/// under test.
pub fn run_and_compare(
    rom_path: &str,
    expected_png: &str,
    config: Config,
    frames: u64,
) -> Result<DiffReport, String> {
    let reference = RefImage::load_png(expected_png)?;
    let mut h = Harness::new(rom_path, config)?;

    const FRAME_CYCLES: u64 = 70_224;
    if h.run_until(StopCondition::Frames(frames), (frames + 20) * FRAME_CYCLES)
        == RunOutcome::TimedOut
    {
        return Err(format!(
            "{rom_path}: did not reach {frames} frames in budget"
        ));
    }
    compare_frame(h.framebuffer(), &reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Color;
    use std::collections::HashSet;

    fn project_config() -> Option<Config> {
        let s = fs::read_to_string("./config.toml").ok()?;
        let mut config: Config = toml::from_str(&s).ok()?;
        config.headless = true;
        config.mode = GBMode::DMG;

        // Mealybug palette
        config.ppu_config.palette.dark = Color::new(0x000000);
        config.ppu_config.palette.dark_gray = Color::new(0x555555);
        config.ppu_config.palette.light_gray = Color::new(0xAAAAAA);
        config.ppu_config.palette.light = Color::new(0xFFFFFF);
        config.ppu_config.palette.off = Color::new(0xFF0000);

        Some(config)
    }

    /// DMG-blob mealybug ROMs paired with their reference PNGs. Any pair whose
    /// files are absent is skipped, so this list can name more than the working
    /// tree carries.
    const MEALYBUG_DMG_BLOB: &[&str] = &[
        "m2_win_en_toggle",
        "m3_bgp_change",
        "m3_bgp_change_sprites",
        "m3_obp0_change",
        "m3_scy_change",
        "m3_scx_low_3_bits",
        "m3_scx_high_5_bits",
        "m3_lcdc_bg_en_change",
        "m3_lcdc_bg_map_change",
        "m3_lcdc_obj_en_change",
        "m3_lcdc_obj_size_change",
        "m3_lcdc_obj_size_change_scx",
        "m3_lcdc_tile_sel_change",
        "m3_lcdc_tile_sel_win_change",
        "m3_lcdc_win_map_change",
        "m3_window_timing",
        "m3_window_timing_wx_0",
        "m3_wx_4_change",
        "m3_wx_5_change",
        "m3_wx_6_change",
        "m3_wx_4_change_sprites",
    ];

    fn dmg_blob_config() -> Option<Config> {
        let mut config = project_config()?;
        config.mode = GBMode::DMG;
        Some(config)
    }

    /// Reporting dashboard: prints each DMG-blob ROM's shade-rank match %, the
    /// frame/reference distinct-shade counts, and the first differing pixel.
    /// Run with `cargo test mealybug_dmg_blob_report -- --nocapture`.
    #[test]
    fn mealybug_dmg_blob_report() {
        let Some(config) = dmg_blob_config() else {
            eprintln!("skipping: no ./config.toml with boot-rom paths");
            return;
        };

        println!("\n{:<32}  {:>8}  first-diff", "rom", "match");
        for name in MEALYBUG_DMG_BLOB {
            let rom = format!("roms/mealybug/{name}.gb");
            let png = format!("roms/mealybug/expected/DMG-blob/{name}.png");
            if !Path::new(&rom).exists() || !Path::new(&png).exists() {
                continue;
            }
            match run_and_compare(&rom, &png, config.clone(), 120) {
                Ok(d) => {
                    let diff = d
                        .first_diff
                        .map(|(x, y)| format!("({x},{y})"))
                        .unwrap_or_else(|| "exact".into());
                    println!("{name:<32}  {:>7.2}%  {diff}", d.match_pct());
                }
                Err(e) => println!("{name:<32}  {:>8}  {e}", "err"),
            }
        }
    }

    /// Diagnostic (run explicitly): drive blargg ROMs that report over the
    /// serial port and print their verdict. Used to confirm interrupt/timing
    /// changes against blargg's hardware-measured expectations.
    #[test]
    fn blargg_serial_report() {
        let Some(config) = project_config() else {
            eprintln!("skipping: no ./config.toml with boot-rom paths");
            return;
        };
        for rom in [
            "roms/blargg/interrupt_time/interrupt_time.gb",
            "roms/blargg/cpu_instrs/individual/02-interrupts.gb",
        ] {
            if !Path::new(rom).exists() {
                continue;
            }
            const FC: u64 = 70_224;
            let mut h = Harness::new(rom, config.clone()).unwrap();
            h.run_until(StopCondition::Frames(400), 420 * FC);
            println!(
                "== {rom}\n{}",
                String::from_utf8_lossy(h.serial()).trim_end()
            );
        }
    }

    /// Mooneye acceptance ROMs, grouped. A test passes when it leaves the
    /// Fibonacci signature (B,C,D,E,H,L = 3,5,8,13,21,34) in the registers
    /// after signalling completion with its `ld b,b` magic breakpoint. Paths
    /// are relative to `roms/moonsuite/`; missing files are skipped.
    const MOONEYE_ACCEPTANCE: &[&str] = &[
        // PPU / STAT timing — the group most relevant to the window and
        // mode-2 STAT work.
        "ppu/intr_2_0_timing",
        "ppu/intr_2_mode0_timing",
        "ppu/intr_2_mode3_timing",
        "ppu/intr_2_oam_ok_timing",
        "ppu/intr_1_2_timing-GS",
        "ppu/stat_lyc_onoff",
        "ppu/stat_irq_blocking",
        "ppu/vblank_stat_intr-GS",
        "ppu/hblank_ly_scx_timing-GS",
        "ppu/lcdon_timing-GS",
        "ppu/lcdon_write_timing-GS",
        // Interrupt dispatch / timing.
        "intr_timing",
        "ei_timing",
        "ei_sequence",
        "di_timing-GS",
        "rapid_di_ei",
        "reti_intr_timing",
        "reti_timing",
        "halt_ime0_ei",
        "halt_ime1_timing",
        "if_ie_registers",
        // Instruction / memory timing.
        "add_sp_e_timing",
        "call_timing",
        "call_cc_timing",
        "jp_timing",
        "jp_cc_timing",
        "ret_timing",
        "ret_cc_timing",
        "push_timing",
        "pop_timing",
        "ld_hl_sp_e_timing",
        "div_timing",
        "rst_timing",
        // OAM DMA.
        "oam_dma_timing",
        "oam_dma_start",
        "oam_dma_restart",
        "oam_dma/basic",
        "oam_dma/reg_read",
    ];

    /// Focused single-ROM diagnostic: run one mooneye ROM to its magic
    /// breakpoint and print the register signature and PC. Edit `ROM` to
    /// target a specific failing test. Ignored by default.
    #[test]
    #[ignore]
    fn mooneye_probe_one() {
        let Some(config) = project_config() else {
            eprintln!("skipping: no ./config.toml with boot-rom paths");
            return;
        };
        const FC: u64 = 70_224;
        const ROM: &str = "roms/moonsuite/acceptance/add_sp_e_timing.gb";
        if !Path::new(ROM).exists() {
            eprintln!("missing {ROM}");
            return;
        }
        let mut h = Harness::new(ROM, config).unwrap();
        let outcome = h.run_until(StopCondition::MagicBreak, 240 * FC);
        println!("outcome={outcome:?} regs={}", h.cpu_regs());
    }

    /// Reporting dashboard: run each mooneye acceptance ROM until it executes
    /// its `LD B,B` magic breakpoint, then read the register signature. PASS
    /// iff B,C,D,E,H,L = 3,5,8,13,21,34 (Fibonacci). If the breakpoint is
    /// never reached inside the budget the row is marked TIMEOUT. Run with
    /// `cargo test mooneye_acceptance_report -- --nocapture`.
    #[test]
    fn mooneye_acceptance_report() {
        let Some(config) = project_config() else {
            eprintln!("skipping: no ./config.toml with boot-rom paths");
            return;
        };

        const FC: u64 = 70_224;

        let mut passed = 0usize;
        let mut ran = 0usize;
        println!("\n{:<40}  {}", "mooneye acceptance", "result");
        for name in MOONEYE_ACCEPTANCE {
            let rom = format!("roms/moonsuite/acceptance/{name}.gb");
            if !Path::new(&rom).exists() {
                println!("Failed to find ROM {rom}");
                continue;
            }
            ran += 1;
            let mut h = match Harness::new(&rom, config.clone()) {
                Ok(h) => h,
                Err(e) => {
                    println!("{name:<40}  err: {e}");
                    continue;
                }
            };

            // Run until the ROM signals completion with `LD B,B`. Mooneye
            // tests finish within a few frames; 240 frames is a generous cap.
            let outcome = h.run_until(StopCondition::MagicBreak, 240 * FC);
            if outcome == RunOutcome::TimedOut {
                println!("{name:<40}  TIMEOUT (no ld b,b)");
                continue;
            }

            let regs = h.cpu_regs();
            if regs.b == 3
                && regs.c == 5
                && regs.d == 8
                && regs.e == 13
                && regs.h == 21
                && regs.l == 34
            {
                passed += 1;
                println!("{name:<40}  PASS");
            } else {
                println!("{name:<40}  FAIL {}", regs);
            }
        }
        println!("\nmooneye acceptance: {passed}/{ran} passed");
    }

    /// Tuning diagnostic (run with `--ignored`): dump one ROM's first rows as
    /// raw shade values (0/85/170/255 → 0..3), ours vs the reference, so a
    /// window/BG divergence can be read column-by-column.
    #[test]
    #[ignore]
    fn window_row_dump() {
        let Some(config) = dmg_blob_config() else {
            eprintln!("skipping: no ./config.toml with boot-rom paths");
            return;
        };
        let name = "m2_win_en_toggle";
        let rom = format!("roms/mealybug/{name}.gb");
        let png = format!("roms/mealybug/expected/DMG-blob/{name}.png");

        let reference = RefImage::load_png(&png).unwrap();
        let mut h = Harness::new(&rom, config).unwrap();
        const FC: u64 = 70_224;
        h.run_until(StopCondition::Frames(120), 140 * FC);
        let frame = h.framebuffer().to_vec();

        let shade = |v: u8| (v as u16 * 3 / 255) as u8;
        let render = |buf: &[u8], row: usize| -> String {
            (0..40)
                .map(|x| {
                    let i = (row * SCREEN_W + x) * 4;
                    std::char::from_digit(shade(buf[i]) as u32, 10).unwrap()
                })
                .collect()
        };

        println!("\n{name} rows 0..6, cols 0..40 (shade 0..3):");
        for row in 0..6 {
            println!("  row {row} ref : {}", render(&reference.rgba, row));
            println!("  row {row} fifo: {}", render(&frame, row));
        }
    }
}
