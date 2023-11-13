use winit::{event_loop::EventLoop, window::WindowBuilder};
use clap::{Parser};

mod cpu;

#[derive(Parser)]
struct Args {
    rom_path: String
}

fn main() -> Result<(), impl std::error::Error> {
    let args = Args::parse();

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("gb-rs")
        .with_inner_size(winit::dpi::LogicalSize::new(128.0, 128.0))
        .build(&event_loop)
        .unwrap();

    event_loop.run(move |event, elwt| {
        println!("{event:?}");
    })
}
