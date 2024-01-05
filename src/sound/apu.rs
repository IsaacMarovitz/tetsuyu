use bitflags::bitflags;
use crate::memory::Memory;
use crate::sound::sc1::SC1;
use crate::sound::sc2::SC2;
use crate::sound::sc3::{OutputLevel, SC3};
use crate::sound::sc4::SC4;
use crate::sound::synth::Synth;

pub struct APU {
    audio_enabled: bool,
    is_ch_4_on: bool,
    is_ch_3_on: bool,
    is_ch_2_on: bool,
    is_ch_1_on: bool,
    panning: Panning,
    sc1: SC1,
    sc2: SC2,
    sc3: SC3,
    sc4: SC4,
    synth: Synth
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
        const CH1_RIGHT = 0b0000_0000;
    }
}

impl APU {
    pub fn new() -> Self {
        let synth = Synth::new();

        Self {
            audio_enabled: true,
            is_ch_4_on: false,
            is_ch_3_on: false,
            is_ch_2_on: false,
            is_ch_1_on: false,
            panning: Panning::empty(),
            sc1: SC1::new(),
            sc2: SC2::new(),
            sc3: SC3::new(),
            sc4: SC4::new(),
            synth
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        // self.sc1.cycle(cycles);
        self.sc2.cycle(cycles);
        self.sc3.cycle(cycles);
        self.sc4.cycle(cycles);

        let s1_vol = {
            if self.is_ch_1_on {
                self.sc1.volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };

        let s1_duty = {
            match self.sc1.duty_cycle {
                DutyCycle::EIGHTH => 0.125,
                DutyCycle::QUARTER => 0.25,
                DutyCycle::HALF => 0.5,
                DutyCycle::THREE_QUARTERS => 0.75,
                _ => 0.0
            }
        };

        let s2_vol = {
            if self.is_ch_2_on {
                self.sc2.volume as f64 / 0xF as f64
            } else {
                0.0
            }
        };

        let s2_duty = {
            match self.sc2.duty_cycle {
                DutyCycle::EIGHTH => 0.125,
                DutyCycle::QUARTER => 0.25,
                DutyCycle::HALF => 0.5,
                DutyCycle::THREE_QUARTERS => 0.75,
                _ => 0.0
            }
        };

        let s3_vol = {
            if self.is_ch_3_on {
                match self.sc3.output_level {
                    OutputLevel::MUTE => 0.0,
                    OutputLevel::QUARTER => 0.25,
                    OutputLevel::HALF => 0.5,
                    OutputLevel::MAX => 1.0,
                    _ => 0.0
                }
            } else {
                0.0
            }
        };

        self.synth.s1_freq.set_value(131072.0 / (2048.0 - self.sc1.period as f64));
        self.synth.s1_vol.set_value(s1_vol);
        self.synth.s1_duty.set_value(s1_duty);

        self.synth.s2_freq.set_value(131072.0 / (2048.0 - self.sc2.period as f64));
        self.synth.s2_vol.set_value(s2_vol);
        self.synth.s2_duty.set_value(s2_duty);

        self.synth.s3_freq.set_value(65536.0 / (2048.0 - self.sc3.period as f64));
        self.synth.s3_vol.set_value(s3_vol);
    }
}

impl Memory for APU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF26 => ((self.audio_enabled as u8) << 7) |
                      ((self.is_ch_4_on as u8) << 3) |
                      ((self.is_ch_3_on as u8) << 2) |
                      ((self.is_ch_2_on as u8) << 1) |
                      ((self.is_ch_1_on as u8) << 0) | 0x70,
            0xFF25 => self.panning.bits(),
            // TODO: VIN
            0xFF24 => 0x00,
            0xFF10..=0xFF14 => self.sc1.read(a),
            0xFF15..=0xFF19 => self.sc2.read(a),
            0xFF1A..=0xFF1E => self.sc3.read(a),
            0xFF30..=0xFF3F => self.sc3.read(a),
            0xFF20..=0xFF24 => self.sc4.read(a),
            _ => 0xFF
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        let mut set_apu_control = false;

        match a {
            0xFF26 => {
                set_apu_control = true;
                self.audio_enabled = (v >> 7) == 0x01;
            },
            0xFF25 => {
                if self.audio_enabled {
                    self.panning = Panning::from_bits_truncate(v)
                }
            },
            // TODO: VIN
            0xFF24 => {},
            0xFF10..=0xFF14 => {
                if self.audio_enabled {
                    self.sc1.write(a, v)
                }
            },
            0xFF16..=0xFF19 => {
                if self.audio_enabled {
                    self.sc2.write(a, v)
                }
            },
            0xFF1A..=0xFF1E => {
                if self.audio_enabled {
                    self.sc3.write(a, v)
                }
            },
            0xFF30..=0xFF3F => self.sc3.write(a, v),
            0xFF20..=0xFF24 => {
                if self.audio_enabled {
                    self.sc4.write(a, v)
                }
            },
            _ => ()
            // _ => panic!("Write to unsupported APU address ({:#06x})!", a),
        }

        if self.sc1.trigger {
            self.sc1.trigger = false;
            if self.sc1.dac_enabled {
                self.is_ch_1_on = true;
            }
        }

        if self.sc2.trigger {
            self.sc2.trigger = false;
            if self.sc2.dac_enabled {
                self.is_ch_2_on = true;
            }
        }

        if self.sc3.trigger {
            self.sc3.trigger = false;
            if self.sc3.dac_enabled {
                self.is_ch_3_on = true;
            }
        }

        if self.sc4.trigger {
            self.sc4.trigger = false;
            if self.sc4.dac_enabled {
                self.is_ch_4_on = true;
            }
        }

        if set_apu_control {
            if !self.audio_enabled {
                self.is_ch_1_on = false;
                self.is_ch_2_on = false;
                self.is_ch_3_on = false;
                self.is_ch_4_on = false;

                self.panning = Panning::empty();

                self.sc1.clear();
                self.sc2.clear();
                self.sc3.clear();
                self.sc4.clear();
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