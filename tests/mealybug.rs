use std::path::Path;
use common::*;

mod common;

/// Reporting dashboard: prints each DMG-blob ROM's shade-rank match %, the
/// frame/reference distinct-shade counts, and the first differing pixel.
/// Run with `cargo test mealybug_dmg_blob_report -- --nocapture`.
#[test]
fn mealybug_dmg_blob_report() {
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
    
    let Some(config) = project_config() else {
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