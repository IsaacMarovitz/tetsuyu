use std::fs::File;
use std::io::Read;
use winit::{event_loop::EventLoop, window::WindowBuilder};
use clap::{Parser};

mod cpu;

#[derive(Parser)]
struct Args {
    rom_path: String
}

fn main() -> Result<(), impl std::error::Error> {
    let args = Args::parse();
    let mut file = File::open(args.rom_path).expect("No ROM found!");
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer).expect("Failed to read ROM!");

    println!("{:#04X?}", buffer);

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("gb-rs")
        .with_inner_size(winit::dpi::LogicalSize::new(160, 144))
        .build(&event_loop)
        .unwrap();

    event_loop.run(move |event, elwt| {
        // println!("{event:?}");
    })
}
