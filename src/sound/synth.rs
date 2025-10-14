use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SizedSample, StreamConfig};
use fundsp::hacker::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::sound::lfsr_noise::lfsr_noise_controlled;

#[derive(Clone)]
pub struct PulseChannel {
    pub freq: Shared,
    pub vol: Shared,
    pub duty: Shared,
    pub l: Shared,
    pub r: Shared,
}

impl PulseChannel {
    fn new() -> Self {
        Self {
            freq: shared(0.0),
            vol: shared(0.0),
            duty: shared(0.0),
            l: shared(0.0),
            r: shared(0.0),
        }
    }

    pub fn update(&self, freq: f32, vol: f32, duty: f32, pan_left: bool, pan_right: bool) {
        self.freq.set_value(freq);
        self.vol.set_value(vol);
        self.duty.set_value(duty);
        self.l.set_value(if pan_left { 1.0 } else { 0.0 });
        self.r.set_value(if pan_right { 1.0 } else { 0.0 });
    }
}

#[derive(Clone)]
pub struct WaveChannel {
    pub freq: Shared,
    pub vol: Shared,
    pub wave: Arc<AtomicTable>,
    pub l: Shared,
    pub r: Shared,
}

impl WaveChannel {
    fn new() -> Self {
        Self {
            freq: shared(0.0),
            vol: shared(0.0),
            wave: Arc::new(AtomicTable::new(&[0.0; 32])),
            l: shared(0.0),
            r: shared(0.0),
        }
    }

    pub fn update(&self, freq: f32, vol: f32, wave: &[f32], pan_left: bool, pan_right: bool) {
        self.freq.set_value(freq);
        self.vol.set_value(vol);

        for i in 0..wave.len() {
            self.wave.set(i, wave[i]);
        }

        self.l.set_value(if pan_left { 1.0 } else { 0.0 });
        self.r.set_value(if pan_right { 1.0 } else { 0.0 });
    }
}

#[derive(Clone)]
pub struct NoiseChannel {
    pub freq: Shared,
    pub vol: Shared,
    pub width: Shared,
    pub l: Shared,
    pub r: Shared,
}

impl NoiseChannel {
    fn new() -> Self {
        Self {
            freq: shared(0.0),
            vol: shared(0.0),
            width: shared(0.0),
            l: shared(0.0),
            r: shared(0.0),
        }
    }

    pub fn update(&self, freq: f32, vol: f32, width_7bit: bool, pan_left: bool, pan_right: bool) {
        self.freq.set_value(freq);
        self.vol.set_value(vol);
        self.width.set_value(if width_7bit { 1.0 } else { 0.0 });
        self.l.set_value(if pan_left { 1.0 } else { 0.0 });
        self.r.set_value(if pan_right { 1.0 } else { 0.0 });
    }
}

#[derive(Clone)]
pub struct GlobalMix {
    pub l: Shared,
    pub r: Shared,
}

impl GlobalMix {
    fn new() -> Self {
        Self {
            l: shared(0.0),
            r: shared(0.0),
        }
    }

    pub fn update(&self, left: f32, right: f32) {
        self.l.set_value(left);
        self.r.set_value(right);
    }
}

pub struct Synth {
    pub ch1: PulseChannel,
    pub ch2: PulseChannel,
    pub ch3: WaveChannel,
    pub ch4: NoiseChannel,
    pub global: GlobalMix,
}

impl Synth {
    pub fn new() -> Self {
        let host = cpal::default_host();

        let ch1 = PulseChannel::new();
        let ch2 = PulseChannel::new();
        let ch3 = WaveChannel::new();
        let ch4 = NoiseChannel::new();
        let global = GlobalMix::new();

        let device = host
            .default_output_device()
            .expect("Failed to find a default output device");
        let config = device.default_output_config().unwrap();

        match config.sample_format() {
            cpal::SampleFormat::F32 => Synth::run_audio::<f32>(
                ch1.clone(),
                ch2.clone(),
                ch3.clone(),
                ch4.clone(),
                global.clone(),
                device,
                config.into(),
            ),
            cpal::SampleFormat::I16 => Synth::run_audio::<i16>(
                ch1.clone(),
                ch2.clone(),
                ch3.clone(),
                ch4.clone(),
                global.clone(),
                device,
                config.into(),
            ),
            cpal::SampleFormat::U16 => Synth::run_audio::<u16>(
                ch1.clone(),
                ch2.clone(),
                ch3.clone(),
                ch4.clone(),
                global.clone(),
                device,
                config.into(),
            ),
            _ => panic!("Unsupported format"),
        }

        Self {
            ch1,
            ch2,
            ch3,
            ch4,
            global,
        }
    }

    fn run_audio<T>(
        ch1: PulseChannel,
        ch2: PulseChannel,
        ch3: WaveChannel,
        ch4: NoiseChannel,
        global: GlobalMix,
        device: Device,
        config: StreamConfig,
    ) where
        T: SizedSample + FromSample<f32>,
    {
        thread::spawn(move || {
            let sample_rate = config.sample_rate.0 as f64;
            let channels = config.channels as usize;

            let ch1_mono =
                ((var(&ch1.freq) | var(&ch1.duty)) >> pulse()) * var(&ch1.vol) * constant(0.25);
            let ch2_mono =
                ((var(&ch2.freq) | var(&ch2.duty)) >> pulse()) * var(&ch2.vol) * constant(0.25);

            let ch3_synth: AtomicSynth<f32> = AtomicSynth::new(ch3.wave);
            let ch3_mono = var(&ch3.freq) >> An(ch3_synth) * var(&ch3.vol) * constant(0.25);

            let ch4_mono = (var(&ch4.freq) | var(&ch4.width))
                >> lfsr_noise_controlled() * var(&ch4.vol) * constant(0.25);

            let ch1_stereo = ch1_mono >> ((pass() * var(&ch1.l)) ^ (pass() * var(&ch1.r)));
            let ch2_stereo = ch2_mono >> ((pass() * var(&ch2.l)) ^ (pass() * var(&ch2.r)));
            let ch3_stereo = ch3_mono >> ((pass() * var(&ch3.l)) ^ (pass() * var(&ch3.r)));
            let ch4_stereo = ch4_mono >> ((pass() * var(&ch4.l)) ^ (pass() * var(&ch4.r)));

            let total_stereo = ch1_stereo + ch2_stereo + ch3_stereo + ch4_stereo;

            let mut c = total_stereo >> (pass() * var(&global.l) | pass() * var(&global.r));
            c.set_sample_rate(sample_rate);

            let mut next_value = move || c.get_stereo();

            let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

            let stream = device
                .build_output_stream(
                    &config,
                    move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                        Synth::write_data(data, channels, &mut next_value)
                    },
                    err_fn,
                    None,
                )
                .unwrap();
            stream.play().unwrap();

            loop {
                thread::sleep(Duration::from_millis(1000));
            }
        });
    }

    fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f32, f32))
    where
        T: SizedSample + FromSample<f32>,
    {
        for frame in output.chunks_mut(channels) {
            let sample = next_sample();
            let left = T::from_sample(sample.0);
            let right: T = T::from_sample(sample.1);

            for (channel, sample) in frame.iter_mut().enumerate() {
                if channel & 1 == 0 {
                    *sample = left;
                } else {
                    *sample = right;
                }
            }
        }
    }
}
