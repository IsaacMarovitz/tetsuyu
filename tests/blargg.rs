use common::*;
use std::path::Path;
use tetsuyu::components::mode::GBMode;

#[macro_use]
mod common;

fn run_blargg_test(sub_path: &str, mode: GBMode) {
    let rom = format!("blargg/{sub_path}.gb");
    if !Path::new(&rom).exists() {
        return;
    }

    let mut h = setup_harness(&rom, mode).expect("Harness initialization failed");
    h.run_until_unbounded(StopCondition::BlarggStatus);

    let mut text_bytes = Vec::new();
    let mut addr = 0xA004;

    loop {
        let byte = h.peek(addr);
        // Break on null-terminator or open-bus/uninitialized memory
        if byte == 0 || byte == 0xFF {
            break;
        }
        text_bytes.push(byte);
        addr += 1;

        if addr > 0xBFFF {
            break;
        }
    }

    let message = String::from_utf8_lossy(&text_bytes);

    println!("\n--- Blargg Test Output ({}) ---", rom);
    println!("{}", message);
    println!("--------------------------------------");

    // Verify the final result code at $A000 (0x00 indicates success)
    let result_code = h.peek(0xA000);
    assert_eq!(
        result_code, 0x00,
        "Blargg test failed with status code {:#04X}.\nCaptured Output: {}",
        result_code, message
    );
}

fn run_serial_blargg_test(sub_path: &str, mode: GBMode) {
    let rom = format!("roms/blargg/{sub_path}.gb");
    if !Path::new(&rom).exists() {
        return;
    }

    let mut h = setup_harness(&rom, mode).expect("Harness initialization failed");
    h.run_until_unbounded(StopCondition::SerialEndsWithAny(&["Passed", "Failed"]));

    let message = String::from_utf8_lossy(h.serial()).into_owned();

    println!("\n--- Blargg Test Output ({}) ---", rom);
    println!("{}", message);
    println!("--------------------------------------");
    assert!(
        message.contains("Passed"),
        "Blargg test failed. Captured Output: {}",
        message
    );
}

mod blargg {
    use super::*;

    test_suite![
        run_blargg_test,
        // DMG Only
        dmg_sound => ("dmg_sound/dmg_sound", GBMode::DMG),
        oam_bug => ("oam_bug/oam_bug", GBMode::DMG),
        // CGB Only
        cgb_sound => ("cgb_sound/cgb_sound", GBMode::CGB),
        interrupt_time => ("interrupt_time/interrupt_time", GBMode::CGB),
        // Cross-model
        mem_timing2 => ("mem_timing-2/mem_timing", GBMode::DMG),
        mem_timing2_cgb => ("mem_timing-2/mem_timing", GBMode::CGB),
    ];

    test_suite![
        run_serial_blargg_test,
        cpu_instrs => ("cpu_instrs/cpu_instrs", GBMode::DMG),
        cpu_instrs_cgb => ("cpu_instrs/cpu_instrs", GBMode::CGB),
        instr_timing => ("instr_timing/instr_timing", GBMode::DMG),
        instr_timing_cgb => ("instr_timing/instr_timing", GBMode::CGB),
        mem_timing => ("mem_timing/mem_timing", GBMode::DMG),
        mem_timing_cgb => ("mem_timing/mem_timing", GBMode::CGB),
    ];
}
