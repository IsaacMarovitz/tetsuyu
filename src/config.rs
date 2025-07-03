use crate::components::mode::{CCMode, GBMode};
use serde::{Deserialize, Serialize};
use winit::keyboard::{Key, SmolStr};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub window_w: u32,
    pub window_h: u32,
    pub print_serial: bool,
    pub boot_rom: String,
    pub shader_path: String,
    pub mode: GBMode,
    pub ppu_config: PPUConfig,
    pub apu_config: APUConfig,
    pub input: Input,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            window_w: 160 * 2,
            window_h: 144 * 2,
            print_serial: false,
            boot_rom: String::default(),
            shader_path: String::default(),
            mode: GBMode::DMG,
            ppu_config: PPUConfig::new(),
            apu_config: APUConfig::new(),
            input: Input::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct PPUConfig {
    pub palette: Palette,
    pub cc_mode: CCMode,
}

impl PPUConfig {
    pub fn new() -> Self {
        Self {
            palette: Palette::new(),
            cc_mode: CCMode::CGB,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct Palette {
    pub dark: Color,
    pub dark_gray: Color,
    pub light_gray: Color,
    pub light: Color,
    pub off: Color,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            dark: Color::new(0x081810),
            dark_gray: Color::new(0x396139),
            light_gray: Color::new(0x84A563),
            light: Color::new(0xC6DE8C),
            off: Color::new(0xD2E6A6),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct Color {
    pub hex: u32,
}

impl Color {
    pub fn new(hex: u32) -> Self {
        Self { hex }
    }

    pub fn r(&self) -> u8 {
        ((self.hex & 0xFF0000) >> 16) as u8
    }

    pub fn g(&self) -> u8 {
        ((self.hex & 0x00FF00) >> 8) as u8
    }

    pub fn b(&self) -> u8 {
        ((self.hex & 0x0000FF) >> 0) as u8
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct APUConfig {
    pub master_enabled: bool,
    pub ch1_enabled: bool,
    pub ch2_enabled: bool,
    pub ch3_enabled: bool,
    pub ch4_enabled: bool,
}

impl APUConfig {
    pub fn new() -> Self {
        Self {
            master_enabled: true,
            ch1_enabled: true,
            ch2_enabled: true,
            ch3_enabled: true,
            ch4_enabled: true,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Input {
    pub up: Key,
    pub left: Key,
    pub down: Key,
    pub right: Key,
    pub a: Key,
    pub b: Key,
    pub select: Key,
    pub start: Key,
}

impl Input {
    pub fn new() -> Self {
        Self {
            up: Key::Character(SmolStr::new("w")),
            left: Key::Character(SmolStr::new("a")),
            down: Key::Character(SmolStr::new("s")),
            right: Key::Character(SmolStr::new("d")),
            a: Key::Character(SmolStr::new("z")),
            b: Key::Character(SmolStr::new("x")),
            select: Key::Character(SmolStr::new("c")),
            start: Key::Character(SmolStr::new("v")),
        }
    }
}
