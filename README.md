<div align="left">
<h1>tetsuyu</h1>
<i>鉄遊 (tetsu-yū) - A Game Boy emulator, written in Rust.</i>
</div>

<br>

## Features
- Cycle-Accurate CPU
- Decently accurate PPU
- Accurate Audio
- DMG & CGB Support
- Configurable Input
- Configurable Palettes & Shaders
- Cross-Platform
- MBC1/MBC2/MBC3/MBC5 Title Support
- Accurate Color Correction

<img width="500" alt="Pokemon Gold" src="https://github.com/IsaacMarovitz/tetsuyu/assets/42140194/e594ea03-2c76-4756-91c9-4592245c5e56">
<img width="500" alt="Demo" src="https://github.com/IsaacMarovitz/tetsuyu/assets/42140194/d57351ba-00d7-4491-90a5-9ccb2b5cb91e">

## Tests
tetsuyu contains harnessess for running blargg, mealybug, and mooneye test suites.
Most known failures are precise PPU timing issues that won't affect most ROMs. 
Current known failures:

### Blargg
- `oam_bug`

### Mealybug
- `m2_win_en_toggle`
- `m3_lcdc_bg_en_change`
- `m3_lcdc_bg_map_change`
- `m3_lcdc_obj_en_change`
- `m3_lcdc_obj_size_change`
- `m3_lcdc_obj_size_change_scx`
- `m3_lcdc_tile_sel_change`
- `m3_lcdc_tile_sel_win_change`
- `m3_lcdc_win_map_change`
- `m3_scx_high_5_bits`
- `m3_scx_low_3_bits`
- `m3_scy_change`
- `m3_window_timing_wx_0`
- `m3_wx_4_change`
- `m3_wx_4_change_sprites`
- `m3_wx_5_change`
- `m3_wx_6_change`

### Mooneye
- `ppu_intr_2_mode0_timing_sprites`
- `bits_unused_hwio_gs` (DMG-only)
- `di_timing_gs` (DMG-only)
- `halt_ime1_timing2_gs` (DMG-only)
- `ppu_hblank_ly_scx_timing_gs` (DMG-only)
- `ppu_intr_1_2_timing_gs` (DMG-only)
- `ppu_lcdon_timing_gs` (DMG-only)
- `ppu_lcdon_write_timing_gs` (DMG-only)
- `ppu_vblank_stat_intr_gs` (DMG-only)

## Referenced Documentation
- https://gbdev.io/pandocs/
- https://ajoneil.github.io/dmg-timing-spec

## Referenced Implementations
- https://github.com/LIJI32/SameBoy
- https://github.com/mvdnes/rboy
- https://github.com/mohanson/gameboy
