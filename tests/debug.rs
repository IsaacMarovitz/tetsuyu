use common::*;
use tetsuyu::components::prelude::ppu::SCREEN_W;

mod common;
#[test]
#[ignore]
fn window_row_dump() {
    let name = "m2_win_en_toggle";
    let rom = format!("roms/mealybug/{name}.gb");
    let png = format!("roms/mealybug/expected/DMG-blob/{name}.png");

    let Ok(reference) = RefImage::load_png(&png) else { return; };
    let Some(mut h) = setup_harness(&rom, tetsuyu::components::mode::GBMode::DMG) else { return; };

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