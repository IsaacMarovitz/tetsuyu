#[macro_use]
extern crate num_derive;

use crate::components::prelude::*;
use crate::config::{Config, Input};
use crate::context::Context;
use clap::Parser;
use std::fs::File;
use std::io::{Read, Write};
use std::{process, thread};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use pollster::FutureExt;
use wgpu::SurfaceError;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::Key;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::event_loop::EventLoop;
use winit::application::ApplicationHandler;
use winit::window::{Window, WindowId};
use crate::components::prelude::ppu::FRAMEBUFFER_SIZE;

type Framebuffer = Arc<RwLock<[u8; FRAMEBUFFER_SIZE]>>;

mod config;
mod context;
mod components;
mod mbc;
mod sound;

pub const CLOCK_FREQUENCY: u32 = 4_194_304;
pub const STEP_TIME: u32 = 16;
pub const STEP_CYCLES: u32 = (STEP_TIME as f64 / (1000_f64 / CLOCK_FREQUENCY as f64)) as u32;

#[derive(Parser)]
struct Args {
    rom_path: String,
    boot_rom: Option<String>
}

struct App {
    game_name: String,
    context: Option<Arc<Mutex<Context>>>,
    config: Config,
    input_tx: Sender<(JoypadButton, bool)>,
    framebuffer: Framebuffer
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title(format!("tetsuyu - {:}", self.game_name))
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.window_w,
                self.config.window_h,
            ));
        let window = event_loop.create_window(window_attributes).unwrap();

        let context_future = Context::new(Arc::new(window), self.config.shader_path.clone());
        self.context = Some(Arc::new(Mutex::new(context_future.block_on())));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let context_arc = Arc::clone(&self.context.as_ref().unwrap());
        let mut context = context_arc.lock().unwrap();
        let size = context.size;

        match event {
            WindowEvent::RedrawRequested if window_id == context.window().id() => {
                match context.render() {
                    Ok(_) => {}
                    Err(SurfaceError::Lost) => context.resize(size),
                    Err(SurfaceError::OutOfMemory) => event_loop.exit(),
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
                            self.config.clone().input,
                            self.input_tx.clone(),
                        );
                    } else if event.state == ElementState::Released {
                        send_input(
                            event.key_without_modifiers(),
                            false,
                            self.config.clone().input,
                            self.input_tx.clone(),
                        );
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let context_arc = Arc::clone(&self.context.as_ref().unwrap());
        let mut context = context_arc.lock().unwrap();

        let framebuffer = Arc::clone(&self.framebuffer);
        context.update(&*framebuffer.read().unwrap());

        let _ = context.render();
    }
}

fn main() {
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

    let panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        panic(info);
        process::exit(1);
    }));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let (input_tx, input_rx) = mpsc::channel::<(JoypadButton, bool)>();
    let framebuffer: Framebuffer = Arc::new(RwLock::new([0xFF; FRAMEBUFFER_SIZE]));

    let mut app = App {
        game_name: String::from(game_name),
        context: None,
        config: config.clone(),
        input_tx,
        framebuffer: framebuffer.clone()
    };

    // Start CPU
    thread::spawn(move || {
        let mut cpu = CPU::new(buffer, config, framebuffer);
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
                step_zero = step_zero.checked_add(Duration::from_millis(u64::from(STEP_TIME))).unwrap();
                
                if now.checked_duration_since(step_zero).is_some() {
                    step_zero = now;
                }
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
            cpu.mem.cycle(cycles);
        }
    });

    let _ = event_loop.run_app(&mut app);
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
