use bitflags::bitflags;
use crate::memory::Memory;
use crate::sound::sc1::SC1;
use crate::sound::sc2::SC2;

pub struct APU {
    audio_enabled: bool,
    is_ch_4_on: bool,
    is_ch_3_on: bool,
    is_ch_2_on: bool,
    is_ch_1_on: bool,
    panning: Panning,
    sc1: SC1,
    sc2: SC2
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
            sc2: SC2::new()
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
                      ((self.is_ch_1_on as u8) << 0),
            0xFF25 => self.panning.bits(),
            // TODO: VIN
            0xFF24 => 0x00,
            0xFF10..=0xFF14 => self.sc1.read(a),
            0xFF16..=0xFF19 => self.sc2.read(a),
            _ => 0x00
            // _ => panic!("Read to unsupported APU address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF26 => self.audio_enabled = (v >> 7) == 0x01,
            0xFF25 => self.panning = Panning::from_bits_truncate(v),
            // TODO: VIN
            0xFF24 => {},
            0xFF10..=0xFF14 => self.sc1.write(a, v),
            0xFF16..=0xFF19 => self.sc2.write(a, v),
            _ => ()
            // _ => panic!("Write to unsupported APU address ({:#06x})!", a),
        }
    }
}

