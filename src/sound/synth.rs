use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SizedSample, StreamConfig};
use fundsp::hacker::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct Synth {
    pub ch1_freq: Shared,
    pub ch1_vol: Shared,
    pub ch1_duty: Shared,
    pub ch1_l: Shared,
    pub ch1_r: Shared,

    pub ch2_freq: Shared,
    pub ch2_vol: Shared,
    pub ch2_duty: Shared,
    pub ch2_l: Shared,
    pub ch2_r: Shared,

    pub ch3_freq: Shared,
    pub ch3_vol: Shared,
    pub ch3_wave: Arc<AtomicTable>,
    pub ch3_l: Shared,
    pub ch3_r: Shared,

    pub ch4_freq: Shared,
    pub ch4_vol: Shared,
    pub ch4_l: Shared,
    pub ch4_r: Shared,

    pub global_l: Shared,
    pub global_r: Shared,
}

impl Synth {
    pub fn new() -> Self {
        let host = cpal::default_host();

        let ch1_freq = shared(0.0);
        let ch1_vol = shared(0.0);
        let ch1_duty = shared(0.0);
        let ch1_l = shared(0.0);
        let ch1_r = shared(0.0);

        let ch2_freq = shared(0.0);
        let ch2_vol = shared(0.0);
        let ch2_duty = shared(0.0);
        let ch2_l = shared(0.0);
        let ch2_r = shared(0.0);

        let ch3_freq = shared(0.0);
        let ch3_vol = shared(0.0);
        let ch3_wave = Arc::new(AtomicTable::new(&[0.0; 32]));
        let ch3_l = shared(0.0);
        let ch3_r = shared(0.0);

        let ch4_freq = shared(0.0);
        let ch4_vol = shared(0.0);
        let ch4_l = shared(0.0);
        let ch4_r = shared(0.0);

        let global_l = shared(0.0);
        let global_r = shared(0.0);

        let device = host
            .default_output_device()
            .expect("Failed to find a default output device");
        let config = device.default_output_config().unwrap();

        match config.sample_format() {
            cpal::SampleFormat::F32 => Synth::run_audio::<f32>(
                ch1_freq.clone(),
                ch1_vol.clone(),
                ch1_duty.clone(),
                ch1_l.clone(),
                ch1_r.clone(),
                ch2_freq.clone(),
                ch2_vol.clone(),
                ch2_duty.clone(),
                ch2_l.clone(),
                ch2_r.clone(),
                ch3_freq.clone(),
                ch3_vol.clone(),
                ch3_wave.clone(),
                ch3_l.clone(),
                ch3_r.clone(),
                ch4_freq.clone(),
                ch4_vol.clone(),
                ch4_l.clone(),
                ch4_r.clone(),
                global_l.clone(),
                global_r.clone(),
                device,
                config.into(),
            ),
            cpal::SampleFormat::I16 => Synth::run_audio::<i16>(
                ch1_freq.clone(),
                ch1_vol.clone(),
                ch1_duty.clone(),
                ch1_l.clone(),
                ch1_r.clone(),
                ch2_freq.clone(),
                ch2_vol.clone(),
                ch2_duty.clone(),
                ch2_l.clone(),
                ch2_r.clone(),
                ch3_freq.clone(),
                ch3_vol.clone(),
                ch3_wave.clone(),
                ch3_l.clone(),
                ch3_r.clone(),
                ch4_freq.clone(),
                ch4_vol.clone(),
                ch4_l.clone(),
                ch4_r.clone(),
                global_l.clone(),
                global_r.clone(),
                device,
                config.into(),
            ),
            cpal::SampleFormat::U16 => Synth::run_audio::<u16>(
                ch1_freq.clone(),
                ch1_vol.clone(),
                ch1_duty.clone(),
                ch1_l.clone(),
                ch1_r.clone(),
                ch2_freq.clone(),
                ch2_vol.clone(),
                ch2_duty.clone(),
                ch2_l.clone(),
                ch2_r.clone(),
                ch3_freq.clone(),
                ch3_vol.clone(),
                ch3_wave.clone(),
                ch3_l.clone(),
                ch3_r.clone(),
                ch4_freq.clone(),
                ch4_vol.clone(),
                ch4_l.clone(),
                ch4_r.clone(),
                global_l.clone(),
                global_r.clone(),
                device,
                config.into(),
            ),
            _ => panic!("Unsupported format"),
        }

        Self {
            ch1_freq,
            ch1_vol,
            ch1_duty,
            ch1_l,
            ch1_r,

            ch2_freq,
            ch2_vol,
            ch2_duty,
            ch2_l,
            ch2_r,

            ch3_freq,
            ch3_vol,
            ch3_wave,
            ch3_l,
            ch3_r,

            ch4_freq,
            ch4_vol,
            ch4_l,
            ch4_r,

            global_l,
            global_r,
        }
    }

    fn run_audio<T>(
        ch1_freq: Shared,
        ch1_vol: Shared,
        ch1_duty: Shared,
        ch1_l: Shared,
        ch1_r: Shared,
        ch2_freq: Shared,
        ch2_vol: Shared,
        ch2_duty: Shared,
        ch2_l: Shared,
        ch2_r: Shared,
        ch3_freq: Shared,
        ch3_vol: Shared,
        ch3_wave: Arc<AtomicTable>,
        ch3_l: Shared,
        ch3_r: Shared,
        ch4_freq: Shared,
        ch4_vol: Shared,
        ch4_l: Shared,
        ch4_r: Shared,
        global_l: Shared,
        global_r: Shared,
        device: Device,
        config: StreamConfig,
    ) where
        T: SizedSample + FromSample<f32>,
    {
        thread::spawn(move || {
            let sample_rate = config.sample_rate.0 as f64;
            let channels = config.channels as usize;

            let ch1_mono =
                ((var(&ch1_freq) | var(&ch1_duty)) >> pulse()) * var(&ch1_vol) * constant(0.25);
            let ch2_mono =
                ((var(&ch2_freq) | var(&ch2_duty)) >> pulse()) * var(&ch2_vol) * constant(0.25);

            let ch3_synth: AtomicSynth<f32> = AtomicSynth::new(ch3_wave);
            let ch3_mono = var(&ch3_freq) >> An(ch3_synth) * var(&ch3_vol) * constant(0.25);
            let ch4_mono = var(&ch4_freq) >> square() * var(&ch4_vol) * constant(0.25);

            let ch1_stereo = ch1_mono >> ((pass() * var(&ch1_l)) ^ (pass() * var(&ch1_r)));
            let ch2_stereo = ch2_mono >> ((pass() * var(&ch2_l)) ^ (pass() * var(&ch2_r)));
            let ch3_stereo = ch3_mono >> ((pass() * var(&ch3_l)) ^ (pass() * var(&ch3_r)));
            let ch4_stereo = ch4_mono >> ((pass() * var(&ch4_l)) ^ (pass() * var(&ch4_r)));

            let total_stereo = ch1_stereo + ch2_stereo + ch3_stereo + ch4_stereo;

            let mut c = total_stereo >> (pass() * var(&global_l) | pass() * var(&global_r));
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
