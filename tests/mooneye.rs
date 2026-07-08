use common::*;
use std::path::Path;
use tetsuyu::components::mode::GBMode;

#[macro_use]
mod common;

pub fn run_mooneye_test(sub_path: &str, mode: GBMode) {
    let rom = format!("roms/moonsuite/acceptance/{sub_path}.gb");
    if !Path::new(&rom).exists() {
        return;
    }

    let mut h = setup_harness(&rom, mode).expect("Harness initialization failed");
    assert_ne!(
        h.run_until(StopCondition::MagicBreak, 240 * FC),
        RunOutcome::TimedOut,
        "Mooneye test execution timed out."
    );

    let regs = h.cpu_regs();
    assert!(
        regs.b == 3 && regs.c == 5 && regs.d == 8 && regs.e == 13 && regs.h == 21 && regs.l == 34,
        "Mooneye register verification signature failed: {regs}"
    );
}

mod dmg {
    use super::*;

    test_suite![
        run_mooneye_test,
        // PPU / STAT timing
        ppu_intr_2_0_timing => ("ppu/intr_2_0_timing", GBMode::DMG),
        ppu_intr_2_mode0_timing => ("ppu/intr_2_mode0_timing", GBMode::DMG),
        ppu_intr_2_mode0_timing_sprites => ("ppu/intr_2_mode0_timing_sprites", GBMode::DMG),
        ppu_intr_2_mode3_timing => ("ppu/intr_2_mode3_timing", GBMode::DMG),
        ppu_intr_2_oam_ok_timing => ("ppu/intr_2_oam_ok_timing", GBMode::DMG),
        ppu_stat_lyc_onoff => ("ppu/stat_lyc_onoff", GBMode::DMG),
        ppu_stat_irq_blocking => ("ppu/stat_irq_blocking", GBMode::DMG),

        // Bits
        bits_mem_oam => ("bits/mem_oam", GBMode::DMG),
        bits_reg_f => ("bits/reg_f", GBMode::DMG),

        // Interrupt dispatch / timing
        interrupts_ie_push => ("interrupts/ie_push", GBMode::DMG),
        intr_timing => ("intr_timing", GBMode::DMG),
        ei_timing => ("ei_timing", GBMode::DMG),
        ei_sequence => ("ei_sequence", GBMode::DMG),
        rapid_di_ei => ("rapid_di_ei", GBMode::DMG),
        reti_intr_timing => ("reti_intr_timing", GBMode::DMG),
        reti_timing => ("reti_timing", GBMode::DMG),
        halt_ime0_ei => ("halt_ime0_ei", GBMode::DMG),
        halt_ime0_nointr_timing => ("halt_ime0_nointr_timing", GBMode::DMG),
        halt_ime1_timing => ("halt_ime1_timing", GBMode::DMG),
        if_ie_registers => ("if_ie_registers", GBMode::DMG),

        // Instruction / memory timing
        instr_daa => ("instr/daa", GBMode::DMG),
        add_sp_e_timing => ("add_sp_e_timing", GBMode::DMG),
        call_timing => ("call_timing", GBMode::DMG),
        call_timing2 => ("call_timing2", GBMode::DMG),
        call_cc_timing => ("call_cc_timing", GBMode::DMG),
        call_cc_timing2 => ("call_cc_timing2", GBMode::DMG),
        jp_timing => ("jp_timing", GBMode::DMG),
        jp_cc_timing => ("jp_cc_timing", GBMode::DMG),
        ret_timing => ("ret_timing", GBMode::DMG),
        ret_cc_timing => ("ret_cc_timing", GBMode::DMG),
        push_timing => ("push_timing", GBMode::DMG),
        pop_timing => ("pop_timing", GBMode::DMG),
        ld_hl_sp_e_timing => ("ld_hl_sp_e_timing", GBMode::DMG),
        div_timing => ("div_timing", GBMode::DMG),
        rst_timing => ("rst_timing", GBMode::DMG),

        // OAM DMA
        oam_dma_timing => ("oam_dma_timing", GBMode::DMG),
        oam_dma_start => ("oam_dma_start", GBMode::DMG),
        oam_dma_restart => ("oam_dma_restart", GBMode::DMG),
        oam_dma_basic => ("oam_dma/basic", GBMode::DMG),
        oam_dma_reg_read => ("oam_dma/reg_read", GBMode::DMG),

        // Timer
        timer_div_write => ("timer/div_write", GBMode::DMG),
        timer_rapid_toggle => ("timer/rapid_toggle", GBMode::DMG),
        timer_tim00_div_trigger => ("timer/tim00_div_trigger", GBMode::DMG),
        timer_tim00 => ("timer/tim00", GBMode::DMG),
        timer_tim01_div_trigger => ("timer/tim01_div_trigger", GBMode::DMG),
        timer_tim01 => ("timer/tim01", GBMode::DMG),
        timer_tim10_div_trigger => ("timer/tim10_div_trigger", GBMode::DMG),
        timer_tim10 => ("timer/tim10", GBMode::DMG),
        timer_tim11_div_trigger => ("timer/tim11_div_trigger", GBMode::DMG),
        timer_tim11 => ("timer/tim11", GBMode::DMG),
        timer_tima_reload => ("timer/tima_reload", GBMode::DMG),
        timer_tima_write_reloading => ("timer/tima_write_reloading", GBMode::DMG),
        timer_tma_write_reloading => ("timer/tma_write_reloading", GBMode::DMG),

        // DMG-Specific Tests
        ppu_intr_1_2_timing_gs => ("ppu/intr_1_2_timing-GS", GBMode::DMG),
        ppu_vblank_stat_intr_gs => ("ppu/vblank_stat_intr-GS", GBMode::DMG),
        ppu_hblank_ly_scx_timing_gs => ("ppu/hblank_ly_scx_timing-GS", GBMode::DMG),
        ppu_lcdon_timing_gs => ("ppu/lcdon_timing-GS", GBMode::DMG),
        ppu_lcdon_write_timing_gs => ("ppu/lcdon_write_timing-GS", GBMode::DMG),
        bits_unused_hwio_gs => ("bits/unused_hwio-GS", GBMode::DMG),
        halt_ime1_timing2_gs => ("halt_ime1_timing2-GS", GBMode::DMG),
        di_timing_gs => ("di_timing-GS", GBMode::DMG),
    ];
}

mod cgb {
    use super::*;

    test_suite![
        run_mooneye_test,
        // PPU / STAT timing
        ppu_intr_2_0_timing => ("ppu/intr_2_0_timing", GBMode::CGB),
        ppu_intr_2_mode0_timing => ("ppu/intr_2_mode0_timing", GBMode::CGB),
        ppu_intr_2_mode0_timing_sprites => ("ppu/intr_2_mode0_timing_sprites", GBMode::CGB),
        ppu_intr_2_mode3_timing => ("ppu/intr_2_mode3_timing", GBMode::CGB),
        ppu_intr_2_oam_ok_timing => ("ppu/intr_2_oam_ok_timing", GBMode::CGB),
        ppu_stat_lyc_onoff => ("ppu/stat_lyc_onoff", GBMode::CGB),
        ppu_stat_irq_blocking => ("ppu/stat_irq_blocking", GBMode::CGB),

        // Bits
        bits_mem_oam => ("bits/mem_oam", GBMode::CGB),
        bits_reg_f => ("bits/reg_f", GBMode::CGB),

        // Interrupt dispatch / timing
        interrupts_ie_push => ("interrupts/ie_push", GBMode::CGB),
        intr_timing => ("intr_timing", GBMode::CGB),
        ei_timing => ("ei_timing", GBMode::CGB),
        ei_sequence => ("ei_sequence", GBMode::CGB),
        rapid_di_ei => ("rapid_di_ei", GBMode::CGB),
        reti_intr_timing => ("reti_intr_timing", GBMode::CGB),
        reti_timing => ("reti_timing", GBMode::CGB),
        halt_ime0_ei => ("halt_ime0_ei", GBMode::CGB),
        halt_ime0_nointr_timing => ("halt_ime0_nointr_timing", GBMode::CGB),
        halt_ime1_timing => ("halt_ime1_timing", GBMode::CGB),
        if_ie_registers => ("if_ie_registers", GBMode::CGB),

        // Instruction / memory timing
        instr_daa => ("instr/daa", GBMode::CGB),
        add_sp_e_timing => ("add_sp_e_timing", GBMode::CGB),
        call_timing => ("call_timing", GBMode::CGB),
        call_timing2 => ("call_timing2", GBMode::CGB),
        call_cc_timing => ("call_cc_timing", GBMode::CGB),
        call_cc_timing2 => ("call_cc_timing2", GBMode::CGB),
        jp_timing => ("jp_timing", GBMode::CGB),
        jp_cc_timing => ("jp_cc_timing", GBMode::CGB),
        ret_timing => ("ret_timing", GBMode::CGB),
        ret_cc_timing => ("ret_cc_timing", GBMode::CGB),
        push_timing => ("push_timing", GBMode::CGB),
        pop_timing => ("pop_timing", GBMode::CGB),
        ld_hl_sp_e_timing => ("ld_hl_sp_e_timing", GBMode::CGB),
        div_timing => ("div_timing", GBMode::CGB),
        rst_timing => ("rst_timing", GBMode::CGB),

        // OAM DMA
        oam_dma_timing => ("oam_dma_timing", GBMode::CGB),
        oam_dma_start => ("oam_dma_start", GBMode::CGB),
        oam_dma_restart => ("oam_dma_restart", GBMode::CGB),
        oam_dma_basic => ("oam_dma/basic", GBMode::CGB),
        oam_dma_reg_read => ("oam_dma/reg_read", GBMode::CGB),

        // Timer
        timer_div_write => ("timer/div_write", GBMode::CGB),
        timer_rapid_toggle => ("timer/rapid_toggle", GBMode::CGB),
        timer_tim00_div_trigger => ("timer/tim00_div_trigger", GBMode::CGB),
        timer_tim00 => ("timer/tim00", GBMode::CGB),
        timer_tim01_div_trigger => ("timer/tim01_div_trigger", GBMode::CGB),
        timer_tim01 => ("timer/tim01", GBMode::CGB),
        timer_tim10_div_trigger => ("timer/tim10_div_trigger", GBMode::CGB),
        timer_tim10 => ("timer/tim10", GBMode::CGB),
        timer_tim11_div_trigger => ("timer/tim11_div_trigger", GBMode::CGB),
        timer_tim11 => ("timer/tim11", GBMode::CGB),
        timer_tima_reload => ("timer/tima_reload", GBMode::CGB),
        timer_tima_write_reloading => ("timer/tima_write_reloading", GBMode::CGB),
        timer_tma_write_reloading => ("timer/tma_write_reloading", GBMode::CGB),
    ];
}

/// Focused single-ROM diagnostic probe helper (Ignored by cargo test run unless specified)
#[test]
#[ignore]
fn mooneye_probe_one() {
    // Uses the generic runner infrastructure function targeting a specific test path manually
    run_mooneye_test("add_sp_e_timing", GBMode::DMG);
}
