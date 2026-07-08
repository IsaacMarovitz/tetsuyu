use crate::CLOCK_FREQUENCY;
use blip_buf::BlipBuf;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SizedSample, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};
use std::thread;
use std::time::Duration;

const FLUSH_CYCLES: u32 = 4096;
const BLIP_CAPACITY: u32 = 4096;
const RING_CAPACITY: usize = 1 << 14;
const TARGET_LATENCY_MS: f64 = 15.0;
const DRIFT_GAIN: f64 = 0.01;
const MAX_DRIFT_CORRECTION: f64 = 0.005;
const OUTPUT_GAIN: f32 = 1.0 / 960.0;
const DC_BLOCK_HZ: f64 = 20.0;

pub struct MixerProducer {
    left: BlipBuf,
    right: BlipBuf,
    time: u32,
    last_left: i32,
    last_right: i32,
    scratch_l: Vec<i16>,
    scratch_r: Vec<i16>,
    producer: Producer<(f32, f32)>,
    base_sample_rate: f64,
    target_fill: usize,
    hp_coeff: f32,
    hp_prev_in_l: f32,
    hp_prev_out_l: f32,
    hp_prev_in_r: f32,
    hp_prev_out_r: f32,
}

impl MixerProducer {
    fn new(sample_rate: u32, producer: Producer<(f32, f32)>) -> Self {
        let mut left = BlipBuf::new(BLIP_CAPACITY);
        let mut right = BlipBuf::new(BLIP_CAPACITY);
        left.set_rates(CLOCK_FREQUENCY as f64, sample_rate as f64);
        right.set_rates(CLOCK_FREQUENCY as f64, sample_rate as f64);
        let target_fill = ((sample_rate as f64) * TARGET_LATENCY_MS / 1000.0).round() as usize;
        let hp_coeff =
            (1.0 - (2.0 * std::f64::consts::PI * DC_BLOCK_HZ / sample_rate as f64)) as f32;
        Self {
            left,
            right,
            time: 0,
            last_left: 0,
            last_right: 0,
            scratch_l: vec![0i16; BLIP_CAPACITY as usize],
            scratch_r: vec![0i16; BLIP_CAPACITY as usize],
            producer,
            base_sample_rate: sample_rate as f64,
            target_fill: core::cmp::max(target_fill, 1),
            hp_coeff,
            hp_prev_in_l: 0.0,
            hp_prev_out_l: 0.0,
            hp_prev_in_r: 0.0,
            hp_prev_out_r: 0.0,
        }
    }
    
    pub fn feed(&mut self, left: i32, right: i32) {
        if left != self.last_left {
            self.left.add_delta(self.time, left - self.last_left);
            self.last_left = left;
        }
        if right != self.last_right {
            self.right.add_delta(self.time, right - self.last_right);
            self.last_right = right;
        }
        self.time += 1;
        if self.time >= FLUSH_CYCLES {
            self.flush();
        }
    }

    fn flush(&mut self) {
        self.left.end_frame(self.time);
        self.right.end_frame(self.time);
        self.time = 0;

        let n_l = self.left.read_samples(&mut self.scratch_l, false);
        let n_r = self.right.read_samples(&mut self.scratch_r, false);
        let n = n_l.min(n_r);
        for i in 0..n {
            let l = Self::dc_block(
                self.scratch_l[i] as f32 * OUTPUT_GAIN,
                self.hp_coeff,
                &mut self.hp_prev_in_l,
                &mut self.hp_prev_out_l,
            );
            let r = Self::dc_block(
                self.scratch_r[i] as f32 * OUTPUT_GAIN,
                self.hp_coeff,
                &mut self.hp_prev_in_r,
                &mut self.hp_prev_out_r,
            );
            // Drop rather than block the emulation thread if the audio thread
            // has fallen behind; drift correction keeps this rare.
            let _ = self.producer.push((l, r));
        }
        self.apply_drift_correction();
    }

    fn dc_block(x: f32, r: f32, prev_in: &mut f32, prev_out: &mut f32) -> f32 {
        let y = x - *prev_in + r * *prev_out;
        *prev_in = x;
        *prev_out = y;
        y
    }

    fn apply_drift_correction(&mut self) {
        let occupied = RING_CAPACITY - self.producer.slots();
        let error = occupied as f64 - self.target_fill as f64;
        let normalized_error = error / self.target_fill as f64;
        let correction = (1.0 - DRIFT_GAIN * normalized_error)
            .clamp(1.0 - MAX_DRIFT_CORRECTION, 1.0 + MAX_DRIFT_CORRECTION);
        let rate = self.base_sample_rate * correction;
        self.left.set_rates(CLOCK_FREQUENCY as f64, rate);
        self.right.set_rates(CLOCK_FREQUENCY as f64, rate);
    }
}

pub struct Mixer {
    producer: MixerProducer,
}

impl Mixer {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("Failed to find a default output device");
        let config = device.default_output_config().unwrap();
        let sample_rate = config.sample_rate();

        let (producer, consumer) = RingBuffer::<(f32, f32)>::new(RING_CAPACITY);
        let producer = MixerProducer::new(sample_rate, producer);

        match config.sample_format() {
            cpal::SampleFormat::F32 => Mixer::run_audio::<f32>(consumer, device, config.into()),
            cpal::SampleFormat::I16 => Mixer::run_audio::<i16>(consumer, device, config.into()),
            cpal::SampleFormat::U16 => Mixer::run_audio::<u16>(consumer, device, config.into()),
            _ => panic!("Unsupported format"),
        }

        Self { producer }
    }

    pub fn feed(&mut self, left: i32, right: i32) {
        self.producer.feed(left, right);
    }

    fn run_audio<T>(mut consumer: Consumer<(f32, f32)>, device: Device, config: StreamConfig)
    where
        T: SizedSample + FromSample<f32>,
    {
        thread::spawn(move || {
            let channels = config.channels as usize;
            let mut last = (0.0f32, 0.0f32);
            let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

            let stream = device
                .build_output_stream(
                    config,
                    move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                        for frame in data.chunks_mut(channels) {
                            let sample = match consumer.pop() {
                                Ok(s) => {
                                    last = s;
                                    s
                                }
                                Err(_) => last,
                            };
                            for (channel, out) in frame.iter_mut().enumerate() {
                                *out = if channel & 1 == 0 {
                                    T::from_sample(sample.0)
                                } else {
                                    T::from_sample(sample.1)
                                };
                            }
                        }
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
}
