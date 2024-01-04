use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, FromSample, SizedSample};
use fundsp::hacker::*;
use assert_no_alloc::*;

pub struct Synth {
    pub square_one: Shared<f64>,
    pub square_two: Shared<f64>
}

impl Synth {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let square_one = shared(0.0);
        let square_two = shared(0.0);

        let device = host
            .default_output_device()
            .expect("Failed to find a default output device");
        let config = device.default_output_config().unwrap();

        match config.sample_format() {
            cpal::SampleFormat::F32 => Synth::run_audio::<f32>(square_one.clone(), square_two.clone(), device, config.into()),
            cpal::SampleFormat::I16 => Synth::run_audio::<i16>(square_one.clone(), square_two.clone(), device, config.into()),
            cpal::SampleFormat::U16 => Synth::run_audio::<u16>(square_one.clone(), square_two.clone(), device, config.into()),
            _ => panic!("Unsupported format"),
        }

        Self {
            square_one,
            square_two
        }
    }

    fn run_audio<T>(square_one: Shared<f64>, square_two: Shared<f64>, device: Device, config: StreamConfig) where T: SizedSample + FromSample<f64>, {
        tokio::spawn(async move {
            let sample_rate = config.sample_rate.0 as f64;
            let channels = config.channels as usize;

            let mut c = (var(&square_one) >> saw()) + (var(&square_two) >> saw());

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
