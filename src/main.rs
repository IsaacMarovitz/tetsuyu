#[macro_use]
extern crate num_derive;

use crate::components::ppu::ppu::{SCREEN_H, SCREEN_W};
use crate::components::prelude::*;
use crate::config::Config;
use crate::context::Context;
use crate::framebuffer::{create_framebuffer_pair, FramebufferReader};
use crate::mbc::header::{CGBFlag, Header};
use clap::Parser;
use pollster::FutureExt;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{process, thread};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::EventLoop;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, SmolStr};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowId};

mod components;
mod config;
mod context;
mod framebuffer;
mod hw;
mod mbc;
mod sound;

pub const CLOCK_FREQUENCY: u32 = 4_194_304;
pub const STEP_TIME: u32 = 16;
pub const STEP_CYCLES: u32 = (STEP_TIME as f64 / (1000_f64 / CLOCK_FREQUENCY as f64)) as u32;

#[derive(Parser)]
struct Args {
    rom_path: String,
    boot_rom: Option<String>,
}

struct App {
    header: Header,
    context: Option<Context>,
    config: Config,
    input_tx: Sender<(JoypadButton, bool)>,
    framebuffer_reader: FramebufferReader,
    occluded: bool,
    dump_frame: bool
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title(format!("tetsuyu - {:}", self.header.title))
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.window_w,
                self.config.window_h,
            ));
        let window = event_loop.create_window(window_attributes).unwrap();

        let context_future = Context::new(Arc::new(window), self.config.shader_path.clone());
        self.context = Some(context_future.block_on());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let context = self.context.as_mut().unwrap();

        match event {
            WindowEvent::RedrawRequested if window_id == context.window().id() => {
                let frame_data = self.framebuffer_reader.get_latest_frame();

                if self.dump_frame {
                    let file = File::create("./frame.png").unwrap();
                    let w = &mut BufWriter::new(file);
                    let mut encoder = png::Encoder::new(w, SCREEN_W as u32, SCREEN_H as u32);

                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.set_compression(png::Compression::NoCompression);

                    let mut writer = encoder.write_header().unwrap();
                    writer.write_image_data(&frame_data).unwrap();
                    self.dump_frame = false;
                }

                context.update(frame_data);
                context.render();
            }
            WindowEvent::Resized(physical_size) => {
                context.resize(physical_size);
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                event_loop.set_control_flow(if occluded {
                    ControlFlow::Wait
                } else {
                    ControlFlow::Poll
                });
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.repeat {
                    if event.state == ElementState::Pressed {
                        self.send_input(
                            event.key_without_modifiers(),
                            true,
                        );
                    } else if event.state == ElementState::Released {
                        self.send_input(
                            event.key_without_modifiers(),
                            false,
                        );
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.occluded {
            return;
        }

        if let Some(context) = &self.context {
            context.window().request_redraw();
        }
    }
}

impl App {
    pub fn send_input(&mut self, key: Key, pressed: bool) {
        let input = &self.config.input;
        let input_tx = &self.input_tx;

        match key {
            key if key == input.up => input_tx.send((JoypadButton::UP, pressed)).unwrap(),
            key if key == input.left => input_tx.send((JoypadButton::LEFT, pressed)).unwrap(),
            key if key == input.down => input_tx.send((JoypadButton::DOWN, pressed)).unwrap(),
            key if key == input.right => input_tx.send((JoypadButton::RIGHT, pressed)).unwrap(),
            key if key == input.a => input_tx.send((JoypadButton::A, pressed)).unwrap(),
            key if key == input.b => input_tx.send((JoypadButton::B, pressed)).unwrap(),
            key if key == input.select => input_tx.send((JoypadButton::SELECT, pressed)).unwrap(),
            key if key == input.start => input_tx.send((JoypadButton::START, pressed)).unwrap(),
            key if key == input.screenshot => self.dump_frame = true,
            _ => (),
        }
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

    let header = Header::new(buffer.clone());
    println!("{}", header);

    assert!(
        (header.cgb_flag != CGBFlag::CGBOnly) || (config.mode != GBMode::DMG),
        "Cannot run CGB only game in DMG Mode!"
    );

    let panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        panic(info);
        process::exit(1);
    }));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let (input_tx, input_rx) = mpsc::channel::<(JoypadButton, bool)>();
    let (framebuffer_writer, framebuffer_reader) = create_framebuffer_pair();

    let mut app = App {
        header: header.clone(),
        context: None,
        config: config.clone(),
        input_tx,
        framebuffer_reader,
        occluded: false,
        dump_frame: false,
    };

    // Start CPU
    thread::spawn(move || {
        let mut mb =
            hw::motherboard::Motherboard::from_config(buffer, header, config, framebuffer_writer);
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
                step_zero = step_zero
                    .checked_add(Duration::from_millis(u64::from(STEP_TIME)))
                    .unwrap();

                if now.checked_duration_since(step_zero).is_some() {
                    step_zero = now;
                }
            }

            match input_rx.try_recv() {
                Ok(v) => {
                    if v.1 {
                        mb.joypad_down(v.0);
                    } else {
                        mb.joypad_up(v.0);
                    }
                }
                Err(_) => {}
            }

            let cycles = mb.step();
            step_cycles += cycles;
        }
    });

    let _ = event_loop.run_app(&mut app);
}
