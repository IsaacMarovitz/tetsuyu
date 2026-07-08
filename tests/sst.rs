use std::fs;
use std::path::Path;
use serde_json::Value;
use tetsuyu::components::cpu::cpu::Cpu;
use tetsuyu::components::mode::GBMode;
use tetsuyu::components::prelude::Registers;
use tetsuyu::hw::bus::{BusDir, Pins};
use tetsuyu::hw::interrupt::{InterruptController, Interrupts};

mod common;

#[test]
fn run_sst_test() {
    let sst_folder = "sst/v1/".to_string();
    let path = Path::new(&sst_folder);

    let paths = fs::read_dir(&path).unwrap();

    for path in paths {
        let test_path = path.unwrap().path();

        if test_path.extension().unwrap() == "json" {
            let contents = fs::read_to_string(test_path).unwrap();

            let json: Value = serde_json::from_str(&contents).unwrap();
            for test in json.as_array().unwrap() {
                let mut cpu = Cpu::new(GBMode::DMG);
                let mut ic = InterruptController::new();
                let mut pins = Pins::new();
                let mut ram: [u8; 0x10000] = [0; 0x10000];

                let name = test["name"].as_str().unwrap();
                let initial_state = &test["initial"];
                let final_state = &test["final"];

                let opcode = u8::from_str_radix(name.split_whitespace().next().unwrap(), 16).ok();
                if matches!(opcode, Some(0x10)) {
                    println!("Skipping STOP test {}", name);
                    continue;
                }

                cpu.reg = json_to_reg(initial_state);

                let ram_values = &initial_state["ram"].as_array().unwrap();
                for ram_val in ram_values.iter() {
                    ram[ram_val[0].as_u64().unwrap() as usize] = ram_val[1].as_u64().unwrap() as u8;
                }

                let cycles = &test["cycles"].as_array().unwrap();

                for _cycle in cycles.iter() {
                    m_cycle(&mut cpu, &mut ic, &mut pins, &mut ram);
                }

                cpu.run_free_acts();

                let final_reg = json_to_reg(final_state);

                let final_values = &final_state["ram"].as_array().unwrap();
                for ram_val in final_values.iter() {
                    let address = ram_val[0].as_u64().unwrap() as usize;
                    let value = ram_val[1].as_u64().unwrap() as u8;

                    assert_eq!(ram[address], value,
                        "{}: RAM at {} was {}, expected {}", name, address, ram[address], value);
                }

                assert_eq!(cpu.reg, final_reg, "{}", name);
            }
        }
    }

    assert!(true);
}

fn m_cycle(cpu: &mut Cpu, ic: &mut InterruptController, mut pins: &mut Pins, ram: &mut [u8; 0x10000]) {
    let was_halted = cpu.is_halted();
    cpu.run_free_acts();

    if cpu.is_halted() {
        if ic.pending() != Interrupts::empty() {
            if !was_halted && !cpu.ime() {
                cpu.trigger_halt_bug();
            }
            cpu.wake();
        } else {
            pins.dir = BusDir::Idle;
            pins.transfer = false;
            return;
        }
    }

    if cpu.take_isr_latch() {
        let pending = ic.pending();
        if let Some(bit) = cpu.latch_isr_vector(pending) {
            ic.acknowledge(bit);
        }
    }

    cpu.setup(&mut pins);

    match pins.dir {
        BusDir::Read => {
            pins.data = ram[pins.address as usize];
        }
        BusDir::Write => {
            ram[pins.address as usize] = pins.data;
        }
        BusDir::Idle => {}
    }

    let fetched = cpu.complete(&pins);
    pins.transfer = false;

    if fetched {
        let pending = ic.pending();
        cpu.offer_interrupt(pending);
    }
}

pub fn json_to_reg(value: &Value) -> Registers {
    Registers {
        a: value["a"].as_u64().unwrap() as u8,
        f: value["f"].as_u64().unwrap() as u8,
        b: value["b"].as_u64().unwrap() as u8,
        c: value["c"].as_u64().unwrap() as u8,
        d: value["d"].as_u64().unwrap() as u8,
        e: value["e"].as_u64().unwrap() as u8,
        h: value["h"].as_u64().unwrap() as u8,
        l: value["l"].as_u64().unwrap() as u8,
        pc: value["pc"].as_u64().unwrap() as u16,
        sp: value["sp"].as_u64().unwrap() as u16,
    }
}