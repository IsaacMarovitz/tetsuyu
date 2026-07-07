use std::path::Path;
use common::*;

mod common;

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