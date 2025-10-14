use crate::components::memory::Memory;
use crate::config::APUConfig;
use crate::sound::prelude::*;
use bitflags::bitflags;

pub struct APU {
    config: APUConfig,
    audio_enabled: bool,
    is_ch_1_active: bool,
    is_ch_2_active: bool,
    is_ch_3_active: bool,
    is_ch_4_active: bool,
    div_apu: u8,
    frame_sequencer: u8,
    left_volume: u8,
    right_volume: u8,
    panning: Panning,
    ch1: CH1,
    ch2: CH2,
    ch3: CH3,
    ch4: CH4,
    synth: Option<Synth>,
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

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct DutyCycle: u8 {
        const EIGHTH = 0b0000_0000;
        const QUARTER = 0b0000_0001;
        const HALF = 0b0000_0010;
        const THREE_QUARTERS = 0b0000_0011;
    }
}

impl APU {
    pub fn new(config: APUConfig) -> Self {
        let synth = if config.master_enabled {
            Some(Synth::new())
        } else {
            None
        };

        Self {
            config,
            audio_enabled: true,
            is_ch_1_active: false,
            is_ch_2_active: false,
            is_ch_3_active: false,
            is_ch_4_active: false,
            div_apu: 0,
            frame_sequencer: 0,
            left_volume: 0,
            right_volume: 0,
            panning: Panning::empty(),
            ch1: CH1::new(),
            ch2: CH2::new(),
            ch3: CH3::new(),
            ch4: CH4::new(),
            synth,
        }
    }

    fn on_div_apu_tick(&mut self) {
        if !self.audio_enabled {
            return;
        }

        self.frame_sequencer = (self.frame_sequencer + 1) & 7;

        // Clock length counters (256Hz)
        if self.frame_sequencer & 1 == 0 {
            if self.ch1.length_counter.tick() {
                self.is_ch_1_active = false;
            }
            if self.ch2.length_counter.tick() {
                self.is_ch_2_active = false;
            }
            if self.ch3.length_counter.tick() {
                self.is_ch_3_active = false;
            }
            if self.ch4.length_counter.tick() {
                self.is_ch_4_active = false;
            }
        }

        // Clock volume envelopes (64Hz)
        if self.frame_sequencer == 7 {
            self.ch1.volume_envelope.tick();
            self.ch2.volume_envelope.tick();
            self.ch4.volume_envelope.tick();
        }

        // Clock sweep (128Hz)
        if self.frame_sequencer == 2 || self.frame_sequencer == 6 {
            self.ch1.tick_sweep();
        }
    }

    pub fn cycle(&mut self, div: u8) {
        let div_bit = (div >> 4) & 1;
        let old_div_bit = (self.div_apu >> 4) & 1;

        if old_div_bit == 1 && div_bit == 0 {
            self.on_div_apu_tick();
        }

        self.div_apu = div;

        // Tick frequency timers (runs every APU cycle)
        if self.is_ch_1_active {
            self.ch1.tick_frequency();
        }
        if self.is_ch_2_active {
            self.ch2.tick_frequency();
        }
        if self.is_ch_3_active {
            self.ch3.tick_frequency();
        }
        if self.is_ch_4_active {
            self.ch4.tick_lfsr();
        }

        if let Some(synth) = &self.synth {
            self.update_channel_1(synth);
            self.update_channel_2(synth);
            self.update_channel_3(synth);
            self.update_channel_4(synth);
            self.update_global_mix(synth);
        }
    }

    fn update_channel_1(&self, synth: &Synth) {
        let vol = if self.is_ch_1_active && self.ch1.dac_enabled && self.config.ch1_enabled {
            self.ch1.volume_envelope.volume / 15.0
        } else {
            0.0
        };

        let duty = match self.ch1.duty_cycle {
            DutyCycle::EIGHTH => 0.125,
            DutyCycle::QUARTER => 0.25,
            DutyCycle::HALF => 0.5,
            DutyCycle::THREE_QUARTERS => 0.75,
            _ => 0.0,
        };

        synth.ch1.update(
            131072.0 / (2048.0 - self.ch1.period as f32),
            vol,
            duty,
            self.panning.contains(Panning::CH1_LEFT),
            self.panning.contains(Panning::CH1_RIGHT),
        );
    }

    fn update_channel_2(&self, synth: &Synth) {
        let vol = if self.is_ch_2_active && self.ch2.dac_enabled && self.config.ch2_enabled {
            self.ch2.volume_envelope.volume / 15.0
        } else {
            0.0
        };

        let duty = match self.ch2.duty_cycle {
            DutyCycle::EIGHTH => 0.125,
            DutyCycle::QUARTER => 0.25,
            DutyCycle::HALF => 0.5,
            DutyCycle::THREE_QUARTERS => 0.75,
            _ => 0.0,
        };

        synth.ch2.update(
            131072.0 / (2048.0 - self.ch2.period as f32),
            vol,
            duty,
            self.panning.contains(Panning::CH2_LEFT),
            self.panning.contains(Panning::CH2_RIGHT),
        );
    }

    fn update_channel_3(&self, synth: &Synth) {
        let vol = if self.is_ch_3_active && self.ch3.dac_enabled && self.config.ch3_enabled {
            let max_sample = match self.ch3.get_volume_shift() {
                0 => 15.0,
                1 => 7.0,
                2 => 3.0,
                _ => 0.0,
            };

            if max_sample > 0.0 {
                max_sample / 15.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        let wave = self.ch3.wave_as_f32();

        synth.ch3.update(
            65536.0 / (2048.0 - self.ch3.period as f32),
            vol,
            &wave,
            self.panning.contains(Panning::CH3_LEFT),
            self.panning.contains(Panning::CH3_RIGHT),
        );
    }

    fn update_channel_4(&self, synth: &Synth) {
        let vol = if self.is_ch_4_active && self.ch4.dac_enabled && self.config.ch4_enabled {
            self.ch4.final_volume / 15.0
        } else {
            0.0
        };

        synth.ch4.update(
            self.ch4.get_frequency(),
            vol,
            self.ch4.is_width_7bit(),
            self.panning.contains(Panning::CH4_LEFT),
            self.panning.contains(Panning::CH4_RIGHT),
        );
    }

    fn update_global_mix(&self, synth: &Synth) {
        let global_l = if self.audio_enabled {
            self.left_volume as f32 / 7.0
        } else {
            0.0
        };

        let global_r = if self.audio_enabled {
            self.right_volume as f32 / 7.0
        } else {
            0.0
        };

        synth.global.update(global_l, global_r);
    }
}

impl Memory for APU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF10..=0xFF14 => self.ch1.read(a),
            0xFF15..=0xFF19 => self.ch2.read(a),
            0xFF1A..=0xFF1E => self.ch3.read(a),
            0xFF1F..=0xFF23 => self.ch4.read(a),
            // NR50: Master Volume & VIN
            0xFF24 => (self.left_volume & 0b0000_0111) << 4 | (self.right_volume & 0b0000_0111),
            // NR51: Sound Panning
            0xFF25 => self.panning.bits(),
            // NR52: Audio Master Control
            0xFF26 => {
                ((self.audio_enabled as u8) << 7)
                    | ((self.is_ch_4_active as u8) << 3)
                    | ((self.is_ch_3_active as u8) << 2)
                    | ((self.is_ch_2_active as u8) << 1)
                    | ((self.is_ch_1_active as u8) << 0)
                    | 0x70
            }
            0xFF30..=0xFF3F => self.ch3.read(a),
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        let mut set_apu_control = false;

        // Ignore writes to 0xFF10-0xFF25 when APU is disabled
        if a >= 0xFF10 && a <= 0xFF25 {
            if !self.audio_enabled {
                return;
            }
        }

        match a {
            0xFF10..=0xFF14 => {
                self.ch1.write(a, v);

                // Handle trigger
                if a == 0xFF14 && (v & 0x80) != 0 {
                    if self.ch1.dac_enabled {
                        self.is_ch_1_active = true;
                    }
                }

                // Handle DAC disable
                if a == 0xFF12 && (v & 0xF8) == 0 {
                    self.is_ch_1_active = false;
                }
            }
            0xFF15..=0xFF19 => {
                self.ch2.write(a, v);

                // Handle trigger
                if a == 0xFF19 && (v & 0x80) != 0 {
                    if self.ch2.dac_enabled {
                        self.is_ch_2_active = true;
                    }
                }

                // Handle DAC disable
                if a == 0xFF17 && (v & 0xF8) == 0 {
                    self.is_ch_2_active = false;
                }
            }
            0xFF1A..=0xFF1E => {
                self.ch3.write(a, v);

                // Handle trigger
                if a == 0xFF1E && (v & 0x80) != 0 {
                    if self.ch3.dac_enabled {
                        self.is_ch_3_active = true;
                    }
                }

                // Handle DAC disable
                if a == 0xFF1A && (v & 0x80) == 0 {
                    self.is_ch_3_active = false;
                }
            }
            0xFF1F..=0xFF23 => {
                self.ch4.write(a, v);

                // Handle trigger
                if a == 0xFF23 && (v & 0x80) != 0 {
                    if self.ch4.dac_enabled {
                        self.is_ch_4_active = true;
                    }
                }

                // Handle DAC disable
                if a == 0xFF21 && (v & 0xF8) == 0 {
                    self.is_ch_4_active = false;
                }
            }
            // NR50: Master Volume & VIN
            0xFF24 => {
                if self.audio_enabled {
                    self.left_volume = v >> 4;
                    self.right_volume = v & 0b0000_0111;
                }
            }
            // NR51: Sound Panning
            0xFF25 => {
                if self.audio_enabled {
                    self.panning = Panning::from_bits_truncate(v)
                }
            }
            // NR52: Audio Master Control
            0xFF26 => {
                set_apu_control = true;
                self.audio_enabled = (v >> 7) == 0x01;
            }
            0xFF30..=0xFF3F => self.ch3.write(a, v),
            _ => panic!("Write to unsupported APU address ({:#06x})!", a),
        }

        if set_apu_control {
            if !self.audio_enabled {
                self.is_ch_1_active = false;
                self.is_ch_2_active = false;
                self.is_ch_3_active = false;
                self.is_ch_4_active = false;
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
