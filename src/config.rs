use serde::{Deserialize, Serialize};
use winit::keyboard::Key;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub window_w: u32,
    pub window_h: u32,
    pub print_serial: bool,
    pub boot_rom: Option<String>,
    pub audio: Audio,
    pub input: Input,
    pub palette: Palette,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            window_w: 160 * 2,
            window_h: 144 * 2,
            print_serial: false,
            boot_rom: None,
            audio: Audio {
                ch1_enabled: true,
                ch2_enabled: true,
                ch3_enabled: true,
                ch4_enabled: true
            },
            input: Input {
                up: Key::Character("w".parse().unwrap()),
                left: Key::Character("a".parse().unwrap()),
                down: Key::Character("s".parse().unwrap()),
                right: Key::Character("d".parse().unwrap()),
                a: Key::Character("z".parse().unwrap()),
                b: Key::Character("x".parse().unwrap()),
                select: Key::Character("c".parse().unwrap()),
                start: Key::Character("v".parse().unwrap()),
            },
            palette: Palette {
                dark: Color { r: 175, g: 203, b: 70 },
                dark_gray: Color { r: 121, g: 170, b: 109 },
                light_gray: Color { r: 34, g: 111, b: 95 },
                light: Color { r: 8, g: 41, b: 85 }
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Palette {
    pub dark: Color,
    pub dark_gray: Color,
    pub light_gray: Color,
    pub light: Color
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Audio {
    pub ch1_enabled: bool,
    pub ch2_enabled: bool,
    pub ch3_enabled: bool,
    pub ch4_enabled: bool
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
    pub start: Key
}