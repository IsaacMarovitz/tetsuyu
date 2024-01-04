use bitflags::bitflags;
use crate::memory::Memory;
use crate::sound::sc1::SC1;
use crate::sound::sc2::SC2;
use crate::sound::sc3::SC3;
use crate::sound::sc4::SC4;

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
    sc4: SC4
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
        Self {
            audio_enabled: false,
            is_ch_4_on: false,
            is_ch_3_on: false,
            is_ch_2_on: false,
            is_ch_1_on: false,
            panning: Panning::empty(),
            sc1: SC1::new(),
            sc2: SC2::new(),
            sc3: SC3::new(),
            sc4: SC4::new()
        }
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
                self.panning = Panning::empty();

                self.sc1.clear();
                self.sc2.clear();
                self.sc3.clear();
                self.sc4.clear();
            }
        }
    }
}

