use common::*;

mod common;

#[test]
fn blargg_serial_report() {
    for rom in [
        "roms/blargg/interrupt_time/interrupt_time.gb",
        "roms/blargg/cpu_instrs/individual/02-interrupts.gb",
    ] {
        let Some(mut h) = setup_harness(rom, tetsuyu::components::mode::GBMode::DMG) else {
            continue;
        };
        h.run_until(StopCondition::Frames(400), 420 * FC);
        println!("== {rom}\n{}", String::from_utf8_lossy(h.serial()).trim_end());
    }
}