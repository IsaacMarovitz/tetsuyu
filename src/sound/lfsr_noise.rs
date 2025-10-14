use fundsp::hacker::*;

#[derive(Clone)]
pub struct LfsrNoiseControlled {
    lfsr: u16,
    phase: f64,
    sample_rate: f64,
}

impl LfsrNoiseControlled {
    pub fn new() -> Self {
        Self {
            lfsr: 0x7FFF,
            phase: 0.0,
            sample_rate: DEFAULT_SR,
        }
    }

    fn step_lfsr(&mut self, width_7bit: bool) {
        let bit_0 = self.lfsr & 1;
        let bit_1 = (self.lfsr >> 1) & 1;

        let xor_result = bit_0 ^ bit_1;

        self.lfsr >>= 1;

        if xor_result != 0 {
            self.lfsr |= 0x4000;
        }

        if width_7bit {
            if xor_result != 0 {
                self.lfsr |= 0x40;
            }
        }
    }
}

impl AudioNode for LfsrNoiseControlled {
    const ID: u64 = 100;
    type Inputs = U2;
    type Outputs = U1;

    fn reset(&mut self) {
        self.lfsr = 0x7FFF;
        self.phase = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let frequency = input[0].max(1.0) as f64;
        let width_7bit = input[1] > 0.5;

        self.phase += frequency / self.sample_rate;

        while self.phase >= 1.0 {
            self.step_lfsr(width_7bit);
            self.phase -= 1.0;
        }

        let output = if self.lfsr & 1 == 0 { 1.0 } else { -1.0 };

        [output].into()
    }
}

pub fn lfsr_noise_controlled() -> An<LfsrNoiseControlled> {
    An(LfsrNoiseControlled::new())
}
