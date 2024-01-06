use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, FromSample, SizedSample};
use fundsp::hacker::*;
use assert_no_alloc::*;

pub struct Synth {
    pub s1_freq: Shared<f64>,
    pub s1_vol: Shared<f64>,
    pub s1_duty: Shared<f64>,
    pub s1_l: Shared<f64>,
    pub s1_r: Shared<f64>,

    pub s2_freq: Shared<f64>,
    pub s2_vol: Shared<f64>,
    pub s2_duty: Shared<f64>,
    pub s2_l: Shared<f64>,
    pub s2_r: Shared<f64>,

    pub s3_freq: Shared<f64>,
    pub s3_vol: Shared<f64>,
    pub s3_l: Shared<f64>,
    pub s3_r: Shared<f64>,

    pub s4_vol: Shared<f64>,
    pub s4_l: Shared<f64>,
    pub s4_r: Shared<f64>,
}

impl Synth {
    pub fn new() -> Self {
        let host = cpal::default_host();

        let s1_freq = shared(0.0);
        let s1_vol = shared(0.0);
        let s1_duty = shared(0.0);
        let s1_l = shared(0.0);
        let s1_r = shared(0.0);

        let s2_freq = shared(0.0);
        let s2_vol = shared(0.0);
        let s2_duty = shared(0.0);
        let s2_l = shared(0.0);
        let s2_r = shared(0.0);

        let s3_freq = shared(0.0);
        let s3_vol = shared(0.0);
        let s3_l = shared(0.0);
        let s3_r = shared(0.0);

        let s4_vol = shared(0.0);
        let s4_l = shared(0.0);
        let s4_r = shared(0.0);

        let device = host
            .default_output_device()
            .expect("Failed to find a default output device");
        let config = device.default_output_config().unwrap();

        match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Synth::run_audio::<f32>(s1_freq.clone(),
                                        s1_vol.clone(),
                                        s1_duty.clone(),
                                        s1_l.clone(),
                                        s1_r.clone(),
                                        s2_freq.clone(),
                                        s2_vol.clone(),
                                        s2_duty.clone(),
                                        s2_l.clone(),
                                        s2_r.clone(),
                                        s3_freq.clone(),
                                        s3_vol.clone(),
                                        s3_l.clone(),
                                        s3_r.clone(),
                                        s4_vol.clone(),
                                        s4_l.clone(),
                                        s4_r.clone(),
                                        device,
                                        config.into())
            },
            cpal::SampleFormat::I16 => {
                Synth::run_audio::<i16>(s1_freq.clone(),
                                        s1_vol.clone(),
                                        s1_duty.clone(),
                                        s1_l.clone(),
                                        s1_r.clone(),
                                        s2_freq.clone(),
                                        s2_vol.clone(),
                                        s2_duty.clone(),
                                        s2_l.clone(),
                                        s2_r.clone(),
                                        s3_freq.clone(),
                                        s3_vol.clone(),
                                        s3_l.clone(),
                                        s3_r.clone(),
                                        s4_vol.clone(),
                                        s4_l.clone(),
                                        s4_r.clone(),
                                        device,
                                        config.into())
            },
            cpal::SampleFormat::U16 => {
                Synth::run_audio::<u16>(s1_freq.clone(),
                                        s1_vol.clone(),
                                        s1_duty.clone(),
                                        s1_l.clone(),
                                        s1_r.clone(),
                                        s2_freq.clone(),
                                        s2_vol.clone(),
                                        s2_duty.clone(),
                                        s2_l.clone(),
                                        s2_r.clone(),
                                        s3_freq.clone(),
                                        s3_vol.clone(),
                                        s3_l.clone(),
                                        s3_r.clone(),
                                        s4_vol.clone(),
                                        s4_l.clone(),
                                        s4_r.clone(),
                                        device,
                                        config.into())
            },
            _ => panic!("Unsupported format"),
        }

        Self {
            s1_freq,
            s1_vol,
            s1_duty,
            s1_l,
            s1_r,

            s2_freq,
            s2_vol,
            s2_duty,
            s2_l,
            s2_r,

            s3_freq,
            s3_vol,
            s3_l,
            s3_r,

            s4_vol,
            s4_l,
            s4_r,
        }
    }

    fn run_audio<T>(
        s1_freq: Shared<f64>,
        s1_vol: Shared<f64>,
        s1_duty: Shared<f64>,
        s1_l: Shared<f64>,
        s1_r: Shared<f64>,
        s2_freq: Shared<f64>,
        s2_vol: Shared<f64>,
        s2_duty: Shared<f64>,
        s2_l: Shared<f64>,
        s2_r: Shared<f64>,
        s3_freq: Shared<f64>,
        s3_vol: Shared<f64>,
        s3_l: Shared<f64>,
        s3_r: Shared<f64>,
        s4_vol: Shared<f64>,
        s4_l: Shared<f64>,
        s4_r: Shared<f64>,
        device: Device,
        config: StreamConfig
    ) where T: SizedSample + FromSample<f64>, {


        tokio::spawn(async move {
            let sample_rate = config.sample_rate.0 as f64;
            let channels = config.channels as usize;

            let sc1_mono = (lfo(move |_| (var(&s1_freq).0.value(), var(&s1_duty).0.value())) >> pulse()) * var(&s1_vol) * constant(0.25);
            let sc2_mono = (lfo(move |_| (var(&s2_freq).0.value(), var(&s2_duty).0.value())) >> pulse()) * var(&s2_vol) * constant(0.25);
            let sc3_mono = var(&s3_freq) >> sine() * var(&s3_vol) * constant(0.25);
            let sc4_mono = noise() * var(&s4_vol) * constant(0.25);

            let sc1_stereo = sc1_mono >> ((pass() * var(&s1_l)) ^ (pass() * var(&s1_r)));
            let sc2_stereo = sc2_mono >> ((pass() * var(&s2_l)) ^ (pass() * var(&s2_r)));
            let sc3_stereo = sc3_mono >> ((pass() * var(&s3_l)) ^ (pass() * var(&s3_r)));
            let sc4_stereo = sc4_mono >> ((pass() * var(&s3_l)) ^ (pass() * var(&s4_r)));

            let mut c = sc1_stereo; //+ sc2_stereo + sc3_stereo; //+ sc4_stereo;

            c.set_sample_rate(sample_rate);
            c.allocate();

            let mut next_value = move || assert_no_alloc(|| c.get_stereo());

            let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

            let stream = device.build_output_stream(
                &config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    Synth::write_data(data, channels, &mut next_value)
                },
                err_fn,
                None,
            ).unwrap();
            stream.play().unwrap();

            loop {
                std::thread::sleep(Duration::from_millis(1));
            }
        });
    }

    fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64)) where T: SizedSample + FromSample<f64>, {
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
