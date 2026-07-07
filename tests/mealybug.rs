use std::path::Path;
use common::*;

#[macro_use]
mod common;

pub fn run_mealybug_test(name: &str) {
    let rom = format!("roms/mealybug/{name}.gb");
    let png = format!("roms/mealybug/expected/DMG-blob/{name}.png");
    if !Path::new(&rom).exists() || !Path::new(&png).exists() { return; }

    match run_and_compare(&rom, &png, 120) {
        Some(Ok(report)) => {
            assert!(
                report.first_diff.is_none(),
                "Framebuffer divergence detected! Match rate: {:.2}%, First diff at: {:?}",
                report.matched as f64 * 100.0 / report.total as f64,
                report.first_diff
            );
        }
        Some(Err(e)) => panic!("Test execution error: {e}"),
        None => {}
    }
}

mod mealybug {
    use super::*;

    test_suite![
        run_mealybug_test,
        m2_win_en_toggle => ("m2_win_en_toggle"),
        m3_bgp_change => ("m3_bgp_change"),
        m3_bgp_change_sprites => ("m3_bgp_change_sprites"),
        m3_obp0_change => ("m3_obp0_change"),
        m3_scy_change => ("m3_scy_change"),
        m3_scx_low_3_bits => ("m3_scx_low_3_bits"),
        m3_scx_high_5_bits => ("m3_scx_high_5_bits"),
        m3_lcdc_bg_en_change => ("m3_lcdc_bg_en_change"),
        m3_lcdc_bg_map_change => ("m3_lcdc_bg_map_change"),
        m3_lcdc_obj_en_change => ("m3_lcdc_obj_en_change"),
        m3_lcdc_obj_size_change => ("m3_lcdc_obj_size_change"),
        m3_lcdc_obj_size_change_scx => ("m3_lcdc_obj_size_change_scx"),
        m3_lcdc_tile_sel_change => ("m3_lcdc_tile_sel_change"),
        m3_lcdc_tile_sel_win_change => ("m3_lcdc_tile_sel_win_change"),
        m3_lcdc_win_map_change => ("m3_lcdc_win_map_change"),
        m3_window_timing => ("m3_window_timing"),
        m3_window_timing_wx_0 => ("m3_window_timing_wx_0"),
        m3_wx_4_change => ("m3_wx_4_change"),
        m3_wx_5_change => ("m3_wx_5_change"),
        m3_wx_6_change => ("m3_wx_6_change"),
        m3_wx_4_change_sprites => ("m3_wx_4_change_sprites"),
    ];
}