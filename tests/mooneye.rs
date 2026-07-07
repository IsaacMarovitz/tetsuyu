use std::path::Path;
use tetsuyu::components::mode::GBMode;
use common::*;

mod common;

/// Expected to pass on CGB and DMG
const MOONEYE_ACCEPTANCE_BASELINE: &[&str] = &[
    // PPU / STAT timing — the group most relevant to the window and
    // mode-2 STAT work.
    "ppu/intr_2_0_timing",
    "ppu/intr_2_mode0_timing",
    "ppu/intr_2_mode0_timing_sprites",
    "ppu/intr_2_mode3_timing",
    "ppu/intr_2_oam_ok_timing",
    "ppu/stat_lyc_onoff",
    "ppu/stat_irq_blocking",
    // Bits
    "bits/mem_oam",
    "bits/reg_f",
    // Interrupt dispatch / timing.
    "interrupts/ie_push",
    "intr_timing",
    "ei_timing",
    "ei_sequence",
    "rapid_di_ei",
    "reti_intr_timing",
    "reti_timing",
    "halt_ime0_ei",
    "halt_ime0_nointr_timing",
    "halt_ime1_timing",
    "if_ie_registers",
    // Instruction / memory timing.
    "instr/daa",
    "add_sp_e_timing",
    "call_timing",
    "call_timing2",
    "call_cc_timing",
    "call_cc_timing2",
    "jp_timing",
    "jp_cc_timing",
    "ret_timing",
    "ret_cc_timing",
    "push_timing",
    "pop_timing",
    "ld_hl_sp_e_timing",
    "div_timing",
    "rst_timing",
    // OAM DMA
    "oam_dma_timing",
    "oam_dma_start",
    "oam_dma_restart",
    "oam_dma/basic",
    "oam_dma/reg_read",
    // Timer
    "timer/div_write",
    "timer/rapid_toggle",
    "timer/tim00_div_trigger",
    "timer/tim00",
    "timer/tim01_div_trigger",
    "timer/tim01",
    "timer/tim10_div_trigger",
    "timer/tim10",
    "timer/tim11_div_trigger",
    "timer/tim11",
    "timer/tima_reload",
    "timer/tima_write_reloading",
    "timer/tma_write_reloading",
];

/// Tests that are only expected to pass on DMG
const MOONEYE_ACCEPTANCE_DMG: &[&str] = &[
    "ppu/intr_1_2_timing-GS",
    "ppu/vblank_stat_intr-GS",
    "ppu/hblank_ly_scx_timing-GS",
    "ppu/lcdon_timing-GS",
    "ppu/lcdon_write_timing-GS",
    "bits/unused_hwio-GS",
    "halt_ime1_timing2-GS",
    "di_timing-GS",
    // TODO: Crashes:
    // "oam_dma/sources-GS",
];

#[test]
fn mooneye_acceptance_report_dmg() {
    let Some(mut config) = project_config() else {
        eprintln!("skipping: no ./config.toml with boot-rom paths");
        return;
    };

    config.mode = GBMode::DMG;

    const FC: u64 = 70_224;

    let mut passed = 0usize;
    let mut ran = 0usize;
    println!("\n{:<40}  {}", "mooneye acceptance DMG", "result");
    for name in [MOONEYE_ACCEPTANCE_BASELINE, MOONEYE_ACCEPTANCE_DMG].concat() {
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

#[test]
fn mooneye_acceptance_report_cgb() {
    let Some(mut config) = project_config() else {
        eprintln!("skipping: no ./config.toml with boot-rom paths");
        return;
    };

    config.mode = GBMode::CGB;

    const FC: u64 = 70_224;

    let mut passed = 0usize;
    let mut ran = 0usize;
    println!("\n{:<40}  {}", "mooneye acceptance CGB", "result");
    for name in MOONEYE_ACCEPTANCE_BASELINE {
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