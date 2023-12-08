use crate::context::Context;
use crate::cpu::CPU;
use crate::mode::GBMode;
use bitflags::Flags;
use clap::Parser;
use std::fs::File;
use std::io::Read;
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

    // Start CPU
    let mut cpu = CPU::new(GBMode::Classic, buffer.clone().try_into().unwrap());

    while true {
        cpu.cycle();
    }

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title(format!("gb-rs - {:}", game_name))
        .with_inner_size(winit::dpi::LogicalSize::new(160, 144))
        .build(&event_loop)
        .unwrap();

    let mut context = Context::new(window).await;

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
