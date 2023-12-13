use crate::context::Context;
use crate::cpu::CPU;
use crate::mode::GBMode;
use clap::Parser;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration, Instant};
use wgpu::SurfaceError;
use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{Key, ModifiersState};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::{event_loop::EventLoop, window::WindowBuilder};
use winit::event_loop::ControlFlow;

mod context;
mod cpu;
mod mmu;
mod mode;
mod registers;
mod ppu;

pub const CLOCK_FREQUENCY: u32 = 4_194_304;
pub const STEP_TIME: u32 = 16;
// STEP_CYCLES = 67108
pub const STEP_CYCLES: u32 = (STEP_TIME as f64 / (1000_f64 / CLOCK_FREQUENCY as f64)) as u32;

#[derive(Parser)]
struct Args {
    rom_path: String,
}

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    let args = Args::parse();
    let mut file = File::open(args.rom_path).expect("No ROM found!");
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer).expect("Failed to read ROM!");

    let nintendo_logo = &buffer[0x0104..=0x0133];

    // Display Nintendo Logo

    // Run checksum

    // Get game name
    let name_data = &buffer[0x0134..=0x0143];
    let index = name_data.iter().position(|&r| r == 0x00).unwrap();
    let game_name = std::str::from_utf8(&name_data[0..index]).expect("Failed to get game name!");
    println!("Starting \"{game_name}\"...");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_title(format!("gb-rs - {:}", game_name))
        .with_inner_size(winit::dpi::LogicalSize::new(ppu::SCREEN_W as u32, ppu::SCREEN_H as u32))
        .build(&event_loop)
        .unwrap();

    let context = Arc::new(Mutex::new(Context::new(window).await));

    {
        let context = Arc::clone(&context);
        // Start CPU
        tokio::spawn(async move {
            let mut cpu = CPU::new(GBMode::Classic, buffer);
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
                    sleep(Duration::from_millis(milliseconds as u64)).await;
                    step_zero = now;
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
        let mut modifiers = ModifiersState::default();
        event_loop.run(move |event, elwt| {
            let mut context = context.lock().unwrap();

            match event {
                Event::AboutToWait => {
                    // TODO: Handle errors
                    let _ = context.render();
                },
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
                        WindowEvent::ModifiersChanged(new) => {
                            modifiers = new.state();
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if event.state == ElementState::Pressed && !event.repeat {
                                match event.key_without_modifiers().as_ref() {
                                    Key::Character("w") => {
                                        println!("Got W Key!");
                                    }
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    }
                },
                _ => ()
            }
        })
    }
}
