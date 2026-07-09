use crate::components::apu::prelude::*;
use crate::components::memory::Memory;
use crate::components::mode::GBMode;
use crate::config::{APUConfig, Config};
use crate::hw::bus::{BusDir, Pins};
use bitflags::bitflags;
use crate::components::prelude::io;

pub struct Apu {
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
    mixer: Option<Mixer>,
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

impl Apu {
    pub fn new(config: Config) -> Self {
        let mixer = if config.apu_config.master_enabled && !config.headless {
            Some(Mixer::new())
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
            mixer,
        }
    }

    pub fn bus(&mut self, pins: &mut Pins) {
        if pins.transfer && matches!(pins.address, 0xFF10..=0xFF3F) {
            match pins.dir {
                BusDir::Read => pins.data = self.read(pins.address),
                BusDir::Write => self.write(pins.address, pins.data),
                BusDir::Idle => {}
            }
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

    pub fn advance(&mut self, div: u8, double_speed: bool) {
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

        if self.mixer.is_some() {
            let (left, right) = self.mix();
            if let Some(mixer) = &mut self.mixer {
                mixer.feed(left, right);
            }
        }
    }

    const DUTY_TABLE: [[u8; 8]; 4] = [
        [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
        [1, 0, 0, 0, 0, 0, 0, 1], // 25%
        [1, 0, 0, 0, 0, 1, 1, 1], // 50%
        [0, 1, 1, 1, 1, 1, 1, 0], // 75%
    ];

    fn mix(&self) -> (i32, i32) {
        if !self.audio_enabled {
            return (0, 0);
        }

        let mut left = 0i32;
        let mut right = 0i32;
        for (digital, pan_l, pan_r) in [
            (self.ch1_digital(), Panning::CH1_LEFT, Panning::CH1_RIGHT),
            (self.ch2_digital(), Panning::CH2_LEFT, Panning::CH2_RIGHT),
            (self.ch3_digital(), Panning::CH3_LEFT, Panning::CH3_RIGHT),
            (self.ch4_digital(), Panning::CH4_LEFT, Panning::CH4_RIGHT),
        ] {
            if let Some(d) = digital {
                let analog = d * 2 - 15;
                if self.panning.contains(pan_l) {
                    left += analog;
                }
                if self.panning.contains(pan_r) {
                    right += analog;
                }
            }
        }

        left *= self.left_volume as i32 + 1;
        right *= self.right_volume as i32 + 1;
        (left, right)
    }

    fn ch1_digital(&self) -> Option<i32> {
        if !(self.ch1.dac_enabled && self.config.ch1_enabled) {
            return None;
        }

        if !self.is_ch_1_active {
            return Some(15);
        }

        let bit = Self::DUTY_TABLE[self.ch1.duty_cycle.bits() as usize]
            [self.ch1.sample_index as usize];

        Some(if bit != 0 {
            self.ch1.volume_envelope.volume as i32
        } else {
            0
        })
    }

    fn ch2_digital(&self) -> Option<i32> {
        if !(self.ch2.dac_enabled && self.config.ch2_enabled) {
            return None;
        }

        if !self.is_ch_2_active {
            return Some(15);
        }

        let bit = Self::DUTY_TABLE[self.ch2.duty_cycle.bits() as usize]
            [self.ch2.sample_index as usize];

        Some(if bit != 0 {
            self.ch2.volume_envelope.volume as i32
        } else {
            0
        })
    }

    fn ch3_digital(&self) -> Option<i32> {
        if !(self.ch3.dac_enabled && self.config.ch3_enabled) {
            return None;
        }

        // Active or DAC-on-but-untriggered: the DAC converts the current wave
        // sample either way (held constant when untriggered).
        Some(self.ch3.wave_digital() as i32)
    }

    fn ch4_digital(&self) -> Option<i32> {
        if !(self.ch4.dac_enabled && self.config.ch4_enabled) {
            return None;
        }

        if !self.is_ch_4_active {
            return Some(15);
        }

        Some(if self.ch4.amplitude_bit() {
            self.ch4.final_volume as i32
        } else {
            0
        })
    }
}

impl Memory for Apu {
    fn read(&self, a: u16) -> u8 {
        match a {
            io::NR10..=io::NR14 => self.ch1.read(a),
            io::NR21..=io::NR24 => self.ch2.read(a),
            io::NR30..=io::NR34 => self.ch3.read(a),
            io::NR41..=io::NR44 => self.ch4.read(a),
            io::NR50 => {
                (self.vin_left as u8) << 7
                    | (self.left_volume & 0b0000_0111) << 4
                    | (self.vin_right as u8) << 3
                    | (self.right_volume & 0b0000_0111)
            }
            io::NR51 => self.panning.bits(),
            io::NR52 => {
                ((self.audio_enabled as u8) << 7)
                    | ((self.is_ch_4_active as u8) << 3)
                    | ((self.is_ch_3_active as u8) << 2)
                    | ((self.is_ch_2_active as u8) << 1)
                    | ((self.is_ch_1_active as u8) << 0)
                    | 0x70
            }
            io::WAV_START..=io::WAV_END => self.ch3.read_wave(a, self.is_ch_3_active, self.mode),
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        let mut set_apu_control = false;

        // While the APU is off, writes to 0xFF10-0xFF25 are ignored. On DMG the
        // length load (low bits of NRx1) is the one exception: it still takes
        // effect, but nothing else in the register does — so load only the
        // length and return, leaving duty and the trigger/enable logic alone.
        if a >= io::NR10 && a <= io::NR51 && !self.audio_enabled {
            if self.mode == GBMode::DMG {
                match a {
                    io::NR11 => self.ch1.length_counter.load((v & 0x3F) as u16, 64),
                    io::NR21 => self.ch2.length_counter.load((v & 0x3F) as u16, 64),
                    io::NR31 => self.ch3.length_counter.load(v as u16, 256),
                    io::NR41 => self.ch4.length_counter.load((v & 0x3F) as u16, 64),
                    _ => {}
                }
            }
            return;
        }

        match a {
            io::NR10..=io::NR14 => {
                let was_enabled = self.ch1.length_counter.enabled;
                let extra = self.length_extra_clock();
                self.ch1.write(a, v);

                if a == io::NR14 {
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
                if a == io::NR12 && (v & 0xF8) == 0 {
                    self.is_ch_1_active = false;
                }

                // A sweep overflow during trigger disables the channel at once.
                if self.ch1.sweep_overflow {
                    self.is_ch_1_active = false;
                    self.ch1.sweep_overflow = false;
                }
            }
            io::NR21..=io::NR24 => {
                let was_enabled = self.ch2.length_counter.enabled;
                let extra = self.length_extra_clock();
                self.ch2.write(a, v);

                if a == io::NR24 {
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
                if a == io::NR22 && (v & 0xF8) == 0 {
                    self.is_ch_2_active = false;
                }
            }
            io::NR30..=io::NR34 => {
                // DMG: retriggering CH3 while active corrupts wave RAM. Must run
                // before the trigger resets the sample index.
                if a == io::NR34
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

                if a == io::NR34 {
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
                if a == io::NR30 && (v & 0x80) == 0 {
                    self.is_ch_3_active = false;
                }
            }
            io::NR41..=io::NR44 => {
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
                if a == io::NR42 && (v & 0xF8) == 0 {
                    self.is_ch_4_active = false;
                }
            }
            io::NR50 => {
                if self.audio_enabled {
                    self.vin_left = (v & 0x80) != 0;
                    self.left_volume = (v >> 4) & 0b0000_0111;
                    self.vin_right = (v & 0x08) != 0;
                    self.right_volume = v & 0b0000_0111;
                }
            }
            io::NR51 => {
                if self.audio_enabled {
                    self.panning = Panning::from_bits_truncate(v)
                }
            }
            io::NR52 => {
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
