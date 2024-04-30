#[macro_use]
extern crate num_derive;

use crate::components::prelude::*;
use crate::config::{Config, Input};
use crate::context::Context;
use clap::Parser;
use std::fs::File;
use std::io::{Read, Write};
use std::{process, thread};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use pollster::FutureExt;
use wgpu::SurfaceError;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::Key;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::{event_loop::EventLoop, window::WindowBuilder};

mod config;
mod context;
mod components;
mod mbc;
mod sound;

pub const CLOCK_FREQUENCY: u32 = 4_194_304;
pub const STEP_TIME: u32 = 16;
// STEP_CYCLES = 67108
pub const STEP_CYCLES: u32 = (STEP_TIME as f64 / (1000_f64 / CLOCK_FREQUENCY as f64)) as u32;

#[derive(Parser)]
struct Args {
    rom_path: String,
    boot_rom: Option<String>,
}

fn main() -> Result<(), impl std::error::Error> {
    let config = match File::open("./config.toml") {
        Ok(mut file) => {
            let mut config_data = String::new();
            file.read_to_string(&mut config_data)
                .expect("Failed to read config!");

            let config = toml::from_str(&config_data).expect("Failed to parse config!");
            config
        }
        Err(_) => {
            let default = Config::default();
            let config = toml::to_string(&default).expect("Failed to serialize config!");
            let mut buffer = File::create("./config.toml").expect("Failed to create file!");
            buffer
                .write_all(config.as_bytes())
                .expect("Failed to write config!");

            default
        }
    };

    let args = Args::parse();
    let mut file = match File::open(args.rom_path.clone()) {
        Ok(file) => file,
        Err(err) => {
            eprintln!(
                "Failed to open ROM at \"{}\": {}",
                args.rom_path.clone(),
                err
            );
            process::exit(1);
        }
    };

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read ROM!");

    // Get game name
    let name_data = &buffer[0x0134..=0x0143];
    let index = name_data.iter().position(|&r| r == 0x00).unwrap();
    let game_name = std::str::from_utf8(&name_data[0..index]).expect("Failed to get game name!");
    println!("Starting \"{}\" in {:?} Mode...", game_name, config.mode);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_title(format!("tetsuyu - {:}", game_name))
        .with_inner_size(winit::dpi::LogicalSize::new(
            config.window_w,
            config.window_h,
        ))
        .build(&event_loop)
        .unwrap();

    let context_future = Context::new(Arc::new(window), config.clone().shader);
    let context = Arc::new(Mutex::new(context_future.block_on()));

    let (input_tx, input_rx) = mpsc::channel::<(JoypadButton, bool)>();

    {
        let config = config.clone();
        let context = Arc::clone(&context);
        // Start CPU
        thread::spawn( move || {
            let mut cpu = CPU::new(buffer, config);
            let mut step_cycles = 0;
            let mut step_zero = Instant::now();

            loop {
                // https://github.com/mohanson/gameboy/blob/master/src/cpu.rs#L13
                if step_cycles > STEP_CYCLES {
                    step_cycles -= STEP_CYCLES;
                    let now = Instant::now();
                    let duration = now.duration_since(step_zero);
                    let milliseconds = STEP_TIME.saturating_sub(duration.as_millis() as u32);
                    // println!("[CPU] Sleeping {}ms", milliseconds);
                    thread::sleep(Duration::from_millis(milliseconds as u64));
                    step_zero = now;
                }

                match input_rx.try_recv() {
                    Ok(v) => {
                        if v.1 {
                            cpu.mem.joypad.down(v.0);
                        } else {
                            cpu.mem.joypad.up(v.0);
                        }
                    }
                    Err(_) => {}
                }

                let cycles = cpu.cycle();
                step_cycles += cycles;
                let did_draw = cpu.mem.cycle(cycles);
                if did_draw {
                    let frame_buffer = cpu.mem.ppu.frame_buffer.clone();
                    let mut context = context.lock().unwrap();
                    context.update(frame_buffer);
                    drop(context);
                }
            }
        });
    }

    {
        let context = Arc::clone(&context);
        event_loop.run(move |event, elwt| {
            let config = config.clone();
            let mut context = context.lock().unwrap();

            match event {
                Event::AboutToWait => {
                    // TODO: Handle errors
                    let _ = context.render();
                }
                Event::WindowEvent { event, window_id } => {
                    let size = context.size;
                    match event {
                        WindowEvent::RedrawRequested if window_id == context.window().id() => {
                            match context.render() {
                                Ok(_) => {}
                                Err(SurfaceError::Lost) => context.resize(size),
                                Err(SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => println!("{:?}", e),
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            context.resize(physical_size);
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if !event.repeat {
                                if event.state == ElementState::Pressed {
                                    send_input(
                                        event.key_without_modifiers(),
                                        true,
                                        config.input,
                                        input_tx.clone(),
                                    );
                                } else if event.state == ElementState::Released {
                                    send_input(
                                        event.key_without_modifiers(),
                                        false,
                                        config.input,
                                        input_tx.clone(),
                                    );
                                }
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        })
    }
}

pub fn send_input(
    key: Key,
    pressed: bool,
    input: Input,
    input_tx: Sender<(JoypadButton, bool)>,
) {
    match key {
        key if key == input.up => input_tx.send((JoypadButton::UP, pressed)).unwrap(),
        key if key == input.left => input_tx.send((JoypadButton::LEFT, pressed)).unwrap(),
        key if key == input.down => input_tx.send((JoypadButton::DOWN, pressed)).unwrap(),
        key if key == input.right => input_tx.send((JoypadButton::RIGHT, pressed)).unwrap(),
        key if key == input.a => input_tx.send((JoypadButton::A, pressed)).unwrap(),
        key if key == input.b => input_tx.send((JoypadButton::B, pressed)).unwrap(),
        key if key == input.select => input_tx.send((JoypadButton::SELECT, pressed)).unwrap(),
        key if key == input.start => input_tx.send((JoypadButton::START, pressed)).unwrap(),
        _ => (),
    }
}
