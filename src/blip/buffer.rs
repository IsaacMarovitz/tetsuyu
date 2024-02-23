#![allow(unused)]
// TODO: Remove this allow

/// A Rust based re-implementation of blip-buf
/// https://code.google.com/p/blip-buf

const BLIP_MAX_RATIO: u32 = 1 << 20;
const BLIP_MAX_FRAME: u32 = 4000;

const TIME_BITS: u32 = 20;
const TIME_UNIT: u32 = 1 << TIME_BITS;
const BASE_SHIFT: u32 = 9;
const END_FRAME_EXTRA: usize = 2;
const HALF_WIDTH: usize = 8;
const BUF_EXTRA: usize = HALF_WIDTH * 2 + END_FRAME_EXTRA;
const PHASE_BITS: u32 = 5;
const PHASE_COUNT: usize = 1 << PHASE_BITS;
const DELTA_BITS: u32 = 15;
const DELTA_UNIT: u32 = 1 << DELTA_BITS;
const FRAC_BITS: u32 = TIME_BITS;
const MAX_SAMPLE: i32 = 32767;
const MIN_SAMPLE: i32 = -32768;

const BL_STEP: [[i32; HALF_WIDTH]; PHASE_COUNT + 1] =
[
[   43, -115,  350, -488, 1136, -914, 5861,21022],
[   44, -118,  348, -473, 1076, -799, 5274,21001],
[   45, -121,  344, -454, 1011, -677, 4706,20936],
[   46, -122,  336, -431,  942, -549, 4156,20829],
[   47, -123,  327, -404,  868, -418, 3629,20679],
[   47, -122,  316, -375,  792, -285, 3124,20488],
[   47, -120,  303, -344,  714, -151, 2644,20256],
[   46, -117,  289, -310,  634,  -17, 2188,19985],
[   46, -114,  273, -275,  553,  117, 1758,19675],
[   44, -108,  255, -237,  471,  247, 1356,19327],
[   43, -103,  237, -199,  390,  373,  981,18944],
[   42,  -98,  218, -160,  310,  495,  633,18527],
[   40,  -91,  198, -121,  231,  611,  314,18078],
[   38,  -84,  178,  -81,  153,  722,   22,17599],
[   36,  -76,  157,  -43,   80,  824, -241,17092],
[   34,  -68,  135,   -3,    8,  919, -476,16558],
[   32,  -61,  115,   34,  -60, 1006, -683,16001],
[   29,  -52,   94,   70, -123, 1083, -862,15422],
[   27,  -44,   73,  106, -184, 1152,-1015,14824],
[   25,  -36,   53,  139, -239, 1211,-1142,14210],
[   22,  -27,   34,  170, -290, 1261,-1244,13582],
[   20,  -20,   16,  199, -335, 1301,-1322,12942],
[   18,  -12,   -3,  226, -375, 1331,-1376,12293],
[   15,   -4,  -19,  250, -410, 1351,-1408,11638],
[   13,    3,  -35,  272, -439, 1361,-1419,10979],
[   11,    9,  -49,  292, -464, 1362,-1410,10319],
[    9,   16,  -63,  309, -483, 1354,-1383, 9660],
[    7,   22,  -75,  322, -496, 1337,-1339, 9005],
[    6,   26,  -85,  333, -504, 1312,-1280, 8355],
[    4,   31,  -94,  341, -507, 1278,-1205, 7713],
[    3,   35, -102,  347, -506, 1238,-1119, 7082],
[    1,   40, -110,  350, -499, 1190,-1021, 6464],
[    0,   43, -115,  350, -488, 1136, -914, 5861]
];

pub struct BlipBuf {
    factor: u32,
    offset: u32,
    avail: usize,
    size: usize,
    integrator: i32,
    samples: Vec<i32>
}

impl BlipBuf {
    pub fn new(size: usize) -> Self {
        let blip = Self {
            factor: TIME_UNIT / BLIP_MAX_RATIO,
            offset: 0,
            avail: 0,
            size,
            integrator: 0,
            samples: vec![0; size + BUF_EXTRA]
        };

        blip
    }

    pub fn clear(&mut self) {
        self.offset = self.factor / 2;
        self.avail = 0;
        self.integrator = 0;
        self.samples = vec![0; self.size + BUF_EXTRA];
    }

    pub fn set_rates(&mut self, clock_rate: u32, sample_rate: u32) {
        // TODO: Fix this
        let factor = (TIME_UNIT as f32 * sample_rate as f32 / clock_rate as f32) as u32;
        self.factor = factor;

        if self.factor < factor {
            self.factor += 1;
        }
    }

    pub fn samples_avail(&self) -> usize {
        self.avail
    }

    pub fn clocks_needed(&self, samples: u32) -> u32 {
        let needed: u32;

        needed = samples * TIME_UNIT;
        if needed < self.offset {
            return 0;
        }

        return (needed - self.offset + self.factor - 1) / self.factor;
    }

    pub fn end_frame(&mut self, t: u32)
    {
        let off = t * self.factor + self.offset;
        self.avail += off as usize >> TIME_BITS;
        self.offset = off & (TIME_UNIT - 1);

        assert!(self.avail <= self.size);
    }

    pub fn remove_samples(&mut self, count: usize) {
        let remain = self.avail + BUF_EXTRA - count;
        self.avail -= count;

        self.samples = self.samples[count..count + remain].to_vec();
        self.samples.append(&mut vec![0; count]);
    }

    pub fn read_samples(&mut self, out: &mut [i32], count: usize, stereo: bool) -> usize {
        let mut count = count;

        if count > self.avail {
            count = self.avail;
        }

        if count != 0 {
            let step = if stereo {
                2
            } else {
                1
            };
            let mut sum = self.integrator;
            let mut in_n = 0;
            let mut out_n = 0;

            while in_n != count {
                let mut s = sum >> DELTA_BITS;

                sum += self.samples[in_n];
                in_n += 1;

                s = s.clamp(MIN_SAMPLE, MAX_SAMPLE);

                out[out_n] = s;
                out_n += step;

                // High Pass Filter
                sum -= s << (DELTA_BITS - BASE_SHIFT);
            }
            self.integrator = sum;

            self.remove_samples(count);
        }

        count
    }

    pub fn add_delta(&mut self, time: u32, mut delta: i32)
    {
        // TODO: Fix this
        let fixed = (time as f32 * self.factor as f32) as u32 + self.offset;
        let out = &mut self.samples[self.avail + (fixed >> FRAC_BITS) as usize..].as_mut();

        let phase_shift = FRAC_BITS - PHASE_BITS;
        let phase: usize = fixed as usize >> phase_shift & (PHASE_COUNT - 1);

        let interp = fixed >> (phase_shift - DELTA_BITS) & (DELTA_UNIT - 1);
        let delta2 = (delta * interp as i32) >> DELTA_BITS;
        delta -= delta2;

        out[0] += BL_STEP[phase][0] * delta + BL_STEP[phase + 1][0] * delta2;
        out[1] += BL_STEP[phase][1] * delta + BL_STEP[phase + 1][1] * delta2;
        out[2] += BL_STEP[phase][2] * delta + BL_STEP[phase + 1][2] * delta2;
        out[3] += BL_STEP[phase][3] * delta + BL_STEP[phase + 1][3] * delta2;
        out[4] += BL_STEP[phase][4] * delta + BL_STEP[phase + 1][4] * delta2;
        out[5] += BL_STEP[phase][5] * delta + BL_STEP[phase + 1][5] * delta2;
        out[6] += BL_STEP[phase][6] * delta + BL_STEP[phase + 1][6] * delta2;
        out[7] += BL_STEP[phase][7] * delta + BL_STEP[phase + 1][7] * delta2;

        out[0] += BL_STEP[PHASE_COUNT - phase][7] * delta + BL_STEP[PHASE_COUNT - phase - 1][7] * delta2;
        out[1] += BL_STEP[PHASE_COUNT - phase][6] * delta + BL_STEP[PHASE_COUNT - phase - 1][6] * delta2;
        out[2] += BL_STEP[PHASE_COUNT - phase][5] * delta + BL_STEP[PHASE_COUNT - phase - 1][5] * delta2;
        out[3] += BL_STEP[PHASE_COUNT - phase][4] * delta + BL_STEP[PHASE_COUNT - phase - 1][4] * delta2;
        out[4] += BL_STEP[PHASE_COUNT - phase][3] * delta + BL_STEP[PHASE_COUNT - phase - 1][3] * delta2;
        out[5] += BL_STEP[PHASE_COUNT - phase][2] * delta + BL_STEP[PHASE_COUNT - phase - 1][2] * delta2;
        out[6] += BL_STEP[PHASE_COUNT - phase][1] * delta + BL_STEP[PHASE_COUNT - phase - 1][1] * delta2;
        out[7] += BL_STEP[PHASE_COUNT - phase][0] * delta + BL_STEP[PHASE_COUNT - phase - 1][0] * delta2;
    }

    pub fn add_delta_fast(&mut self, time: u32, delta: i32)
    {
        let fixed = time * self.factor + self.offset;
        let out = &mut self.samples[self.avail + (fixed >> FRAC_BITS) as usize..].as_mut();

        let interp = fixed >> (FRAC_BITS - DELTA_BITS) & (DELTA_UNIT - 1);
        let delta2 = delta * interp as i32;

        out[7] += delta * DELTA_UNIT as i32 - delta2;
        out[8] += delta2;
    }
}