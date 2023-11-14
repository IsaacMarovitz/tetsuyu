use std::fs::File;
use std::io::Read;
use winit::{event_loop::EventLoop, window::WindowBuilder};
use clap::{Parser};
use crate::cpu::{CPU, GBMode};

mod cpu;
mod registers;
mod mode;

#[derive(Parser)]
struct Args {
    rom_path: String
}

fn main() -> Result<(), impl std::error::Error> {
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
    let cpu = CPU::new(GBMode::Classic);

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title(format!("gb-rs - {:}", game_name))
        .with_inner_size(winit::dpi::LogicalSize::new(160, 144))
        .build(&event_loop)
        .unwrap();

    event_loop.run(move |event, elwt| {
        // println!("{event:?}");
    })
}
