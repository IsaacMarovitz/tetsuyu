//! Cycle-accurate, band-limited synthesis for channel 3.
//!
//! Channel 3's wave RAM can be rewritten fast enough to play arbitrary PCM
//! audio (as several commercial and homebrew ROMs do), which pushes its
//! effective "note frequency" at or beyond the host output's Nyquist
//! frequency. A plain phase-accumulator wavetable oscillator has no way to
//! represent that without severe aliasing, since it assumes the table
//! content is a static, low-frequency periodic waveform.

use crate::CLOCK_FREQUENCY;
use crate::sound::ch3::CH3;
use blip_buf::BlipBuf;
use fundsp::prelude::*;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{Arc, Mutex};

const FLUSH_CYCLES: u32 = 4096;
const BLIP_CAPACITY: u32 = 4096;
const RING_CAPACITY: usize = 1 << 14;
const TARGET_LATENCY_MS: f64 = 15.0;
const DRIFT_GAIN: f64 = 0.01;
const MAX_DRIFT_CORRECTION: f64 = 0.005;

pub struct Ch3BlipProducer {
    blip: BlipBuf,
    time: u32,
    last_amplitude: i32,
    scratch: Vec<i16>,
    producer: Producer<f32>,
    base_sample_rate: f64,
    target_fill: usize,
}

impl Ch3BlipProducer {
    fn new(sample_rate: u32, producer: Producer<f32>) -> Self {
        let mut blip = BlipBuf::new(BLIP_CAPACITY);
        blip.set_rates(CLOCK_FREQUENCY as f64, sample_rate as f64);
        let target_fill = ((sample_rate as f64) * TARGET_LATENCY_MS / 1000.0).round() as usize;
        Self {
            blip,
            time: 0,
            last_amplitude: 0,
            scratch: vec![0i16; BLIP_CAPACITY as usize],
            producer,
            base_sample_rate: sample_rate as f64,
            target_fill: core::cmp::max(target_fill, 1),
        }
    }

    pub fn feed(&mut self, amplitude: i32) {
        if amplitude != self.last_amplitude {
            self.blip
                .add_delta(self.time, amplitude - self.last_amplitude);
            self.last_amplitude = amplitude;
        }
        self.time += 1;
        if self.time >= FLUSH_CYCLES {
            self.flush();
        }
    }

    fn flush(&mut self) {
        self.blip.end_frame(self.time);
        self.time = 0;
        while self.blip.samples_avail() > 0 {
            let n = self.blip.read_samples(&mut self.scratch, false);
            for &s in &self.scratch[..n] {
                // If the audio thread has fallen behind, drop the sample
                // rather than block the emulation thread; the drift
                // correction below aims to make this rare, and the ring
                // buffer's own headroom absorbs the rest.
                let _ = self.producer.push(s as f32 / CH3::AMPLITUDE_SCALE as f32);
            }
        }
        self.apply_drift_correction();
    }

    fn apply_drift_correction(&mut self) {
        let occupied = RING_CAPACITY - self.producer.slots();
        let error = occupied as f64 - self.target_fill as f64;
        let normalized_error = error / self.target_fill as f64;
        let correction = (1.0 - DRIFT_GAIN * normalized_error)
            .clamp(1.0 - MAX_DRIFT_CORRECTION, 1.0 + MAX_DRIFT_CORRECTION);
        self.blip
            .set_rates(CLOCK_FREQUENCY as f64, self.base_sample_rate * correction);
    }
}

#[derive(Clone)]
pub struct Ch3BlipNode {
    consumer: Arc<Mutex<Consumer<f32>>>,
    last_output: f32,
}

impl AudioNode for Ch3BlipNode {
    const ID: u64 = 101;

    type Inputs = U0;
    type Outputs = U1;

    #[inline]
    fn tick(&mut self, _input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        if let Ok(mut consumer) = self.consumer.lock() {
            if let Ok(sample) = consumer.pop() {
                self.last_output = sample;
            }
        }
        [self.last_output].into()
    }

    fn route(&mut self, input: &SignalFrame, _frequency: f64) -> SignalFrame {
        Routing::Generator(0.0).route(input, self.outputs())
    }
}

pub fn ch3_blip_pair(sample_rate: u32) -> (Ch3BlipProducer, Ch3BlipNode) {
    let (producer, consumer) = RingBuffer::<f32>::new(RING_CAPACITY);
    let producer = Ch3BlipProducer::new(sample_rate, producer);
    let node = Ch3BlipNode {
        consumer: Arc::new(Mutex::new(consumer)),
        last_output: 0.0,
    };
    (producer, node)
}
