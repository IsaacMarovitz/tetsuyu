#![allow(unused)]
// TODO: Remove this allow

use crate::components::memory::Memory;
use crate::sound::prelude::*;
use crate::blip::buffer::BlipBuf;
use bitflags::bitflags;
use crate::CLOCK_FREQUENCY;

struct Blip {
    data: BlipBuf,
    from: u32,
    ampl: i32
}

impl Blip {
    pub fn new(data: BlipBuf) -> Self {
        Self {
            data,
            from: 0,
            ampl: 0
        }
    }

    fn set(&mut self, time: u32, ampl: i32) {
        self.from = time;
        let delta = ampl - self.ampl;
        self.ampl = ampl;
        self.data.add_delta(time, delta);
    }
}

pub struct APU {
    audio_enabled: bool,
    is_ch_4_on: bool,
    is_ch_3_on: bool,
    is_ch_2_on: bool,
    is_ch_1_on: bool,
    left_volume: u8,
    right_volume: u8,
    panning: Panning,
    ch1: CH1,
    ch2: CH2,
    ch3: CH3,
    ch4: CH4,
    div_one: bool,
    freq: f64,
    blip: Blip
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct Panning: u8 {
        const CH4_LEFT = 0b1000_0000;
        const CH3_LEFT = 0b0100_0000;
        const CH2_LEFT = 0b0010_0000;
        const CH1_LEFT = 0b0001_0000;
        const CH4_RIGHT = 0b0000_1000;
        const CH3_RIGHT = 0b0000_0100;
        const CH2_RIGHT = 0b0000_0010;
        const CH1_RIGHT = 0b0000_0001;
    }
}

impl APU {
    pub fn new(sample_rate: usize) -> Self {
        let mut blip_buf = BlipBuf::new(sample_rate);
        blip_buf.set_rates(CLOCK_FREQUENCY, sample_rate as u32);
        let blip = Blip::new(blip_buf);

        Self {
            audio_enabled: true,
            is_ch_4_on: false,
            is_ch_3_on: false,
            is_ch_2_on: false,
            is_ch_1_on: true,
            left_volume: 0,
            right_volume: 0,
            panning: Panning::empty(),
            ch1: CH1::new(),
            ch2: CH2::new(),
            ch3: CH3::new(),
            ch4: CH4::new(),
            div_one: false,
            freq: 256.0,
            blip
        }
    }

    pub fn cycle(&mut self, div: u8) {
        let mut div_tick = false;

        if self.div_one {
            // TODO: Double-speed mode
            if div & (0b000_1000) == 0 {
                // Bit moved from 1 -> 0
                div_tick = true;
                self.div_one = false;
            }
        } else {
            if div & (0b000_1000) >> 3 == 1 {
                self.div_one = true;
            }
        }

        if div_tick {
            self.ch1.cycle();
            self.ch2.cycle();
            self.ch3.cycle();
            self.ch4.cycle();
        }

        let ch1_vol = {
            if self.ch1.dac_enabled {
                self.ch1.volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };

        let ch1_duty = {
            match self.ch1.duty_cycle {
                DutyCycle::EIGHTH => 0.125,
                DutyCycle::QUARTER => 0.25,
                DutyCycle::HALF => 0.5,
                DutyCycle::THREE_QUARTERS => 0.75,
                _ => 0.0,
            }
        };

        let ch2_vol = {
            if self.ch2.dac_enabled {
                self.ch2.volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };

        let ch2_duty = {
            match self.ch2.duty_cycle {
                DutyCycle::EIGHTH => 0.125,
                DutyCycle::QUARTER => 0.25,
                DutyCycle::HALF => 0.5,
                DutyCycle::THREE_QUARTERS => 0.75,
                _ => 0.0,
            }
        };

        let ch3_vol = {
            if self.ch3.dac_enabled {
                match self.ch3.output_level {
                    OutputLevel::MUTE => 0.0,
                    OutputLevel::QUARTER => 0.25,
                    OutputLevel::HALF => 0.5,
                    OutputLevel::MAX => 1.0,
                    _ => 0.0,
                }
            } else {
                0.0
            }
        };

        let ch4_vol = {
            if self.ch4.dac_enabled {
                self.ch4.final_volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };

        // TODO: Amplifier on original hardware NEVER completely mutes non-silent input
        let global_l = {
            if self.audio_enabled {
                self.left_volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };

        let global_r = {
            if self.audio_enabled {
                self.right_volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };
    }
}

impl Memory for APU {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR52: Audio Master Control
            0xFF26 => {
                ((self.audio_enabled as u8) << 7)
                    | ((self.is_ch_4_on as u8) << 3)
                    | ((self.is_ch_3_on as u8) << 2)
                    | ((self.is_ch_2_on as u8) << 1)
                    | ((self.is_ch_1_on as u8) << 0)
                    | 0x70
            }
            // NR51: Sound Panning
            0xFF25 => self.panning.bits(),
            // NR50: Master Volume & VIN
            0xFF24 => (self.left_volume & 0b0000_0111) << 4 | (self.right_volume & 0b0000_0111),
            0xFF10..=0xFF14 => self.ch1.read(a),
            0xFF15..=0xFF19 => self.ch2.read(a),
            0xFF1A..=0xFF1E => self.ch3.read(a),
            0xFF30..=0xFF3F => self.ch3.read(a),
            0xFF20..=0xFF24 => self.ch4.read(a),
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        let mut set_apu_control = false;

        match a {
            // NR52: Audio Master Control
            0xFF26 => {
                set_apu_control = true;
                self.audio_enabled = (v >> 7) == 0x01;
            }
            // NR51: Sound Panning
            0xFF25 => {
                if self.audio_enabled {
                    self.panning = Panning::from_bits_truncate(v)
                }
            }
            // NR50: Master Volume & VIN
            0xFF24 => {
                if self.audio_enabled {
                    self.left_volume = v >> 4;
                    self.right_volume = v & 0b0000_0111;
                }
            }
            0xFF10..=0xFF14 => {
                if self.audio_enabled {
                    self.ch1.write(a, v)
                }
            }
            0xFF16..=0xFF19 => {
                if self.audio_enabled {
                    self.ch2.write(a, v)
                }
            }
            0xFF1A..=0xFF1E => {
                if self.audio_enabled {
                    self.ch3.write(a, v)
                }
            }
            0xFF30..=0xFF3F => self.ch3.write(a, v),
            0xFF20..=0xFF24 => {
                if self.audio_enabled {
                    self.ch4.write(a, v)
                }
            }
            _ => (), // _ => panic!("Write to unsupported APU address ({:#06x})!", a),
        }

        if self.ch1.trigger {
            self.ch1.trigger = false;
            if self.ch1.dac_enabled {
                self.is_ch_1_on = true;
            }
        }

        if self.ch2.trigger {
            self.ch2.trigger = false;
            if self.ch2.dac_enabled {
                self.is_ch_2_on = true;
            }
        }

        if self.ch3.trigger {
            self.ch3.trigger = false;
            if self.ch3.dac_enabled {
                self.is_ch_3_on = true;
            }
        }

        if self.ch4.trigger {
            self.ch4.trigger = false;
            self.ch4.lfsr = 0;
            if self.ch4.dac_enabled {
                self.is_ch_4_on = true;
            }
        }

        if set_apu_control {
            if !self.audio_enabled {
                self.is_ch_1_on = false;
                self.is_ch_2_on = false;
                self.is_ch_3_on = false;
                self.is_ch_4_on = false;
                self.left_volume = 0;
                self.right_volume = 0;

                self.panning = Panning::empty();

                self.ch1.clear();
                self.ch2.clear();
                self.ch3.clear();
                self.ch4.clear();
            }
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct DutyCycle: u8 {
        const EIGHTH = 0b0000_0000;
        const QUARTER = 0b0000_0001;
        const HALF = 0b0000_00010;
        const THREE_QUARTERS = 0b0000_0011;
    }
}
