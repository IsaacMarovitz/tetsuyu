use crate::components::memory::Memory;
use crate::config::{APUConfig, Config};
use crate::sound::prelude::*;
use bitflags::bitflags;
use crate::components::mode::GBMode;

pub struct APU {
    config: APUConfig,
    mode: GBMode,
    audio_enabled: bool,
    is_ch_1_active: bool,
    is_ch_2_active: bool,
    is_ch_3_active: bool,
    is_ch_4_active: bool,
    div_apu: u8,
    frame_sequencer: u8,
    left_volume: u8,
    right_volume: u8,
    vin_left: bool,
    vin_right: bool,
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
    pub fn new(config: Config) -> Self {
        let synth = if config.apu_config.master_enabled && !config.headless {
            Some(Synth::new())
        } else {
            None
        };

        Self {
            config: config.apu_config,
            mode: config.mode,
            audio_enabled: true,
            is_ch_1_active: false,
            is_ch_2_active: false,
            is_ch_3_active: false,
            is_ch_4_active: false,
            div_apu: 0,
            frame_sequencer: 0,
            left_volume: 0,
            right_volume: 0,
            vin_left: false,
            vin_right: false,
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
            if self.ch1.sweep_overflow {
                self.is_ch_1_active = false;
                self.ch1.sweep_overflow = false;
            }
        }
    }

    fn length_extra_clock(&self) -> bool {
        // The next frame-sequencer step won't clock length while the sequencer
        // sits on an even step, so enabling length now gives one extra clock.
        self.frame_sequencer & 1 == 0
    }

    pub fn cycle(&mut self, div: u8, double_speed: bool) {
        let bit = 4 + double_speed as u8;
        let div_bit = (div >> bit) & 1;
        let old_div_bit = (self.div_apu >> bit) & 1;

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
        // Channel 3 is fed as cycle-accurate DAC amplitude events (see
        // `sound::ch3_blip`) rather than a frequency + wavetable snapshot, so
        // PCM-style wave RAM content is reproduced without aliasing. Report
        // 0 while inactive so the on/off transition itself is captured as a
        // proper band-limited step.
        let active = self.is_ch_3_active && self.ch3.dac_enabled && self.config.ch3_enabled;
        let amplitude = if active { self.ch3.current_amplitude() } else { 0 };

        synth.ch3.feed(
            amplitude,
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
            (self.left_volume + 1) as f32 / 8.0
        } else {
            0.0
        };

        let global_r = if self.audio_enabled {
            (self.right_volume + 1) as f32 / 8.0
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
            0xFF24 => {
                (self.vin_left as u8) << 7
                    | (self.left_volume & 0b0000_0111) << 4
                    | (self.vin_right as u8) << 3
                    | (self.right_volume & 0b0000_0111)
            }
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
            0xFF30..=0xFF3F => self.ch3.read_wave(a, self.is_ch_3_active, self.mode),
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        let mut set_apu_control = false;

        // While the APU is off, writes to 0xFF10-0xFF25 are ignored. On DMG the
        // length load (low bits of NRx1) is the one exception: it still takes
        // effect, but nothing else in the register does — so load only the
        // length and return, leaving duty and the trigger/enable logic alone.
        if a >= 0xFF10 && a <= 0xFF25 && !self.audio_enabled {
            if self.mode == GBMode::DMG {
                match a {
                    0xFF11 => self.ch1.length_counter.load((v & 0x3F) as u16, 64),
                    0xFF16 => self.ch2.length_counter.load((v & 0x3F) as u16, 64),
                    0xFF1B => self.ch3.length_counter.load(v as u16, 256),
                    0xFF20 => self.ch4.length_counter.load((v & 0x3F) as u16, 64),
                    _ => {}
                }
            }
            return;
        }

        match a {
            0xFF10..=0xFF14 => {
                let was_enabled = self.ch1.length_counter.enabled;
                let extra = self.length_extra_clock();
                self.ch1.write(a, v);

                if a == 0xFF14 {
                    let trigger = (v & 0x80) != 0;
                    let hit_zero = self.ch1.length_counter.enable_clock(was_enabled, extra);
                    if hit_zero && !trigger {
                        self.is_ch_1_active = false;
                    }
                    if trigger {
                        self.ch1.length_counter.trigger_reload(64, extra);
                        if self.ch1.dac_enabled {
                            self.is_ch_1_active = true;
                        }
                    }
                }

                // Handle DAC disable
                if a == 0xFF12 && (v & 0xF8) == 0 {
                    self.is_ch_1_active = false;
                }

                // A sweep overflow during trigger disables the channel at once.
                if self.ch1.sweep_overflow {
                    self.is_ch_1_active = false;
                    self.ch1.sweep_overflow = false;
                }
            }
            0xFF15..=0xFF19 => {
                let was_enabled = self.ch2.length_counter.enabled;
                let extra = self.length_extra_clock();
                self.ch2.write(a, v);

                if a == 0xFF19 {
                    let trigger = (v & 0x80) != 0;
                    let hit_zero = self.ch2.length_counter.enable_clock(was_enabled, extra);
                    if hit_zero && !trigger {
                        self.is_ch_2_active = false;
                    }
                    if trigger {
                        self.ch2.length_counter.trigger_reload(64, extra);
                        if self.ch2.dac_enabled {
                            self.is_ch_2_active = true;
                        }
                    }
                }

                // Handle DAC disable
                if a == 0xFF17 && (v & 0xF8) == 0 {
                    self.is_ch_2_active = false;
                }
            }
            0xFF1A..=0xFF1E => {
                // DMG: retriggering CH3 while active corrupts wave RAM. Must run
                // before the trigger resets the sample index.
                if a == 0xFF1E
                    && (v & 0x80) != 0
                    && self.mode == GBMode::DMG
                    && self.is_ch_3_active
                    && self.ch3.about_to_read()
                {
                    self.ch3.corrupt_wave_ram();
                }

                let was_enabled = self.ch3.length_counter.enabled;
                let extra = self.length_extra_clock();
                self.ch3.write(a, v);

                if a == 0xFF1E {
                    let trigger = (v & 0x80) != 0;
                    let hit_zero = self.ch3.length_counter.enable_clock(was_enabled, extra);
                    if hit_zero && !trigger {
                        self.is_ch_3_active = false;
                    }
                    if trigger {
                        self.ch3.length_counter.trigger_reload(256, extra);
                        if self.ch3.dac_enabled {
                            self.is_ch_3_active = true;
                        }
                    }
                }

                // Handle DAC disable
                if a == 0xFF1A && (v & 0x80) == 0 {
                    self.is_ch_3_active = false;
                }
            }
            0xFF1F..=0xFF23 => {
                let was_enabled = self.ch4.length_counter.enabled;
                let extra = self.length_extra_clock();
                self.ch4.write(a, v);

                if a == 0xFF23 {
                    let trigger = (v & 0x80) != 0;
                    let hit_zero = self.ch4.length_counter.enable_clock(was_enabled, extra);
                    if hit_zero && !trigger {
                        self.is_ch_4_active = false;
                    }
                    if trigger {
                        self.ch4.length_counter.trigger_reload(64, extra);
                        if self.ch4.dac_enabled {
                            self.is_ch_4_active = true;
                        }
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
                    self.vin_left = (v & 0x80) != 0;
                    self.left_volume = (v >> 4) & 0b0000_0111;
                    self.vin_right = (v & 0x08) != 0;
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
                let was_enabled = self.audio_enabled;
                set_apu_control = true;
                self.audio_enabled = (v >> 7) == 0x01;

                if !was_enabled && self.audio_enabled {
                    self.frame_sequencer = 7;
                }
            }
            0xFF30..=0xFF3F => self.ch3.write_wave(a, v, self.is_ch_3_active, self.mode),
            _ => {}
        }

        if set_apu_control {
            if !self.audio_enabled {
                self.is_ch_1_active = false;
                self.is_ch_2_active = false;
                self.is_ch_3_active = false;
                self.is_ch_4_active = false;
                self.left_volume = 0;
                self.right_volume = 0;
                self.vin_left = false;
                self.vin_right = false;
                self.panning = Panning::empty();

                // On DMG the length counters are unaffected by power; only CGB
                // clears them. Snapshot the counts and restore them after the
                // channel clears when running as DMG.
                let preserve_length = self.mode == GBMode::DMG;
                let lengths = [
                    self.ch1.length_counter.counter,
                    self.ch2.length_counter.counter,
                    self.ch3.length_counter.counter,
                    self.ch4.length_counter.counter,
                ];

                self.ch1.clear();
                self.ch2.clear();
                self.ch3.clear();
                self.ch4.clear();

                if preserve_length {
                    self.ch1.length_counter.counter = lengths[0];
                    self.ch2.length_counter.counter = lengths[1];
                    self.ch3.length_counter.counter = lengths[2];
                    self.ch4.length_counter.counter = lengths[3];
                }
            }
        }
    }
}
