use serde::{Deserialize, Serialize};
use winit::keyboard::{Key, SmolStr};
use crate::components::mode::{CCMode, GBMode};

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
            input: Input::new()
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
            cc_mode: CCMode::CGB
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct Palette {
    pub dark: Color,
    pub dark_gray: Color,
    pub light_gray: Color,
    pub light: Color,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            dark: Color::new(175, 203, 70),
            dark_gray: Color::new(121, 170, 109),
            light_gray: Color::new(34, 111, 95),
            light: Color::new(8, 41, 95)
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
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
            ch4_enabled: true
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