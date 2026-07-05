use crate::components::memory::Memory;
use crate::components::mode::GBMode;
use crate::sound::length_counter::LengthCounter;
use crate::sound::period_timer::PeriodTimer;
use bitflags::bitflags;

pub struct CH3 {
    pub dac_enabled: bool,
    pub output_level: OutputLevel,
    pub period: u16,
    wave_ram: [u8; 16],
    timer: PeriodTimer,
    pub sample_index: u8,
    just_fetched: bool,
    pub length_counter: LengthCounter,
}

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct OutputLevel: u8 {
        const MUTE = 0b0000_0000;
        const MAX = 0b0010_0000;
        const HALF = 0b0100_0000;
        const QUARTER = 0b0110_0000;
    }
}

impl CH3 {
    pub fn new() -> Self {
        Self {
            dac_enabled: false,
            output_level: OutputLevel::MUTE,
            period: 0,
            wave_ram: [0; 16],
            timer: PeriodTimer::new(),
            sample_index: 0,
            just_fetched: false,
            length_counter: LengthCounter::new(),
        }
    }

    pub fn clear(&mut self) {
        self.dac_enabled = false;
        self.output_level = OutputLevel::MUTE;
        self.period = 0;
        self.timer.set(0);
        self.sample_index = 0;
        self.just_fetched = false;
        self.length_counter.clear();
    }

    pub fn tick_frequency(&mut self) {
        // Reload period: (2048 - period) * 2. Advance one of 32 samples.
        // `just_fetched` marks the single tick on which a new sample byte is
        // read from wave RAM; the DMG only exposes wave RAM to the CPU then.
        self.just_fetched = false;
        if self.timer.tick((2048 - self.period) * 2) {
            self.sample_index = (self.sample_index + 1) & 0x1F;
            self.just_fetched = true;
        }
    }

    pub fn trigger(&mut self) {
        // The wave channel waits an extra 6 T-cycles after a trigger before it
        // fetches the first sample, so its position lags a plain reload.
        self.timer.set((2048 - self.period) * 2 + 6);

        // Reset sample position
        self.sample_index = 0;
        self.just_fetched = false;
    }

    pub fn corrupt_wave_ram(&mut self) {
        // DMG only: triggering CH3 while it is about to read a wave-RAM byte
        // rewrites the low bytes. sample_index counts nibbles, so the byte
        // it is about to read is (sample_index / 2), advanced by one because
        // the trigger lands just before the next read.
        let byte = ((self.sample_index as usize + 1) & 0x1F) / 2;
        if byte < 4 {
            self.wave_ram[0] = self.wave_ram[byte];
        } else {
            let base = byte & 0x0C;
            for i in 0..4 {
                self.wave_ram[i] = self.wave_ram[base + i];
            }
        }
    }

    /// Wave-RAM read. While the channel is on, the CGB returns the byte the
    /// channel is currently reading (any address maps to it). The DMG only
    /// exposes that byte during the single tick it fetches a sample, blocking
    /// with 0xFF otherwise. While off, wave RAM is directly addressable.
    pub fn read_wave(&self, a: u16, active: bool, mode: GBMode) -> u8 {
        if !active {
            self.wave_ram[a as usize - 0xFF30]
        } else if mode == GBMode::CGB || self.just_fetched {
            self.wave_ram[(self.sample_index >> 1) as usize]
        } else {
            0xFF
        }
    }

    /// Wave-RAM write. Mirrors `read_wave`: while the channel is on, the CGB
    /// redirects the write to the byte currently being read, and the DMG only
    /// accepts it during the single tick it fetches a sample; while off, wave
    /// RAM is directly addressable.
    pub fn write_wave(&mut self, a: u16, v: u8, active: bool, mode: GBMode) {
        if !active {
            self.wave_ram[a as usize - 0xFF30] = v;
        } else if mode == GBMode::CGB || self.just_fetched {
            self.wave_ram[(self.sample_index >> 1) as usize] = v;
        }
    }

    pub fn get_volume_shift(&self) -> u8 {
        match self.output_level {
            OutputLevel::MAX => 0,
            OutputLevel::HALF => 1,
            OutputLevel::QUARTER => 2,
            OutputLevel::MUTE => 4,
            _ => 4,
        }
    }

    pub fn wave_as_f32(&self, shift: u8) -> [f32; 32] {
        const U4_MAX: f32 = 0b1111 as f32;
        let mut wave: [f32; 32] = [0f32; 32];

        for i in 0..self.wave_ram.len() {
            let hi = (self.wave_ram[i] & 0b1111_0000) >> 4;
            let lo = self.wave_ram[i] & 0b0000_1111;

            wave[i * 2] = (((hi >> shift) as f32 / U4_MAX) * 2.0) - 1.0;
            wave[(i * 2) + 1] = (((lo >> shift) as f32 / U4_MAX) * 2.0) - 1.0;
        }

        wave
    }
}

impl Memory for CH3 {
    fn read(&self, a: u16) -> u8 {
        match a {
            // NR30: DAC Enable
            0xFF1A => (self.dac_enabled as u8) << 7 | 0x7F,
            // NR31: Length Timer
            0xFF1B => 0xFF,
            // NR32: Output Level
            0xFF1C => self.output_level.bits() | 0x9F,
            // NR33: Period Low
            0xFF1D => 0xFF,
            // NR34: Period High & Control
            0xFF1E => (self.length_counter.enabled as u8) << 6 | 0xBF,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            // NR30: DAC Enable
            0xFF1A => {
                self.dac_enabled = ((v & 0b1000_0000) >> 7) != 0;
            }
            // NR31: Length Timer
            0xFF1B => {
                self.length_counter.load(v as u16, 256);
            }
            // NR32: Output Level
            0xFF1C => {
                self.output_level = OutputLevel::from_bits_truncate(v & 0b0110_0000);
            }
            // NR33: Period Low
            0xFF1D => {
                self.period = (self.period & 0xFF00) | (v as u16);
            }
            // NR34: Period High & Control
            0xFF1E => {
                let trigger = ((v & 0b1000_0000) >> 7) != 0;
                self.length_counter.enabled = ((v & 0b0100_0000) >> 6) != 0;
                self.period = (self.period & 0x00FF) | (((v & 0x07) as u16) << 8);

                if trigger {
                    self.trigger();
                }
            }
            _ => panic!("Write to unsupported CH3 address ({:#06x})!", a),
        }
    }
}
