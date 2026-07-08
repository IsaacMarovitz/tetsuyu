#[macro_use]
extern crate num_derive;

pub mod components;
pub mod config;
pub mod context;
pub mod framebuffer;
pub mod hw;
pub mod mbc;
pub mod sound;

pub const CLOCK_FREQUENCY: u32 = 4_194_304;
pub const STEP_TIME: u32 = 16;
pub const STEP_CYCLES: u32 = (STEP_TIME as f64 / (1000_f64 / CLOCK_FREQUENCY as f64)) as u32;
