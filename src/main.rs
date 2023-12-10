use crate::context::Context;
use crate::cpu::CPU;
use crate::mode::GBMode;
use bitflags::Flags;
use clap::Parser;
use std::fs::File;
use std::io::Read;
use tokio::time::{sleep, Duration};
use wgpu::SurfaceError;
use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{Key, ModifiersState};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::{event_loop::EventLoop, window::WindowBuilder};

mod context;
mod cpu;
mod mmu;
mod mode;
mod registers;
mod gpu;

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

    let window = WindowBuilder::new()
        .with_title(format!("gb-rs - {:}", game_name))
        .with_inner_size(winit::dpi::LogicalSize::new(160, 144))
        .build(&event_loop)
        .unwrap();

    let mut context = Context::new(window).await;

    let buffer_copy = buffer.clone();
    // Start CPU
    tokio::spawn(async move {
        let mut cpu = CPU::new(GBMode::Classic, buffer_copy);

        while true {
            let cycles = cpu.cycle();
            cpu.mem.gpu.cycle();

            sleep(Duration::from_millis((1000_f64 / 4_194_304_f64 * cycles as f64) as u64)).await;
        }
    });

    let mut modifiers = ModifiersState::default();

    event_loop.run(move |event, elwt| {
        if let Event::WindowEvent { event, window_id } = event {
            match event {
                WindowEvent::RedrawRequested if window_id == context.window().id() => {
                    match context.render() {
                        Ok(_) => {}
                        Err(SurfaceError::Lost) => context.resize(context.size),
                        Err(SurfaceError::OutOfMemory) => elwt.exit(),
                        Err(e) => println!("{:?}", e),
                    }
                }
                WindowEvent::Resized(physical_size) => {
                    context.update(vec![0xFF; 40000]);
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
        }
    })
}
