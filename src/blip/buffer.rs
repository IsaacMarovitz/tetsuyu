const BLIP_MAX_RATIO: u32 = 1 << 20;
const BLIP_MAX_FRAME: u32 = 4000;

const TIME_BITS: u32 = 20;
const TIME_UNIT: u32 = 1 << TIME_BITS;
const BASE_SHIFT: u32 = 9;
const END_FRAME_EXTRA: u32 = 2;
const HALF_WIDTH: u32 = 8;
const BUF_EXTRA: u32 = HALF_WIDTH * 2 + END_FRAME_EXTRA;
const PHASE_BITS: u32 = 5;
const PHASE_COUNT: u32 = 1 << PHASE_BITS;
const DELTA_BITS: u32 = 15;
const DELTA_UNIT: u32 = 1 << DELTA_BITS;
const FRAC_BITS: u32 = TIME_BITS;
const MAX_SAMPLE: i32 = 32767;
const MIN_SAMPLE: i32 = -32768;

const BL_STEP: [[i16; (PHASE_COUNT + 1) as usize]; HALF_WIDTH as usize] =
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

struct Blip {
    factor: u32,
    offset: u32,
    avail: u32,
    size: u32,
    integrator: u32
}

impl Blip {
    pub fn new(size: u32) -> Self {
        assert!(size >= 0);

        let blip = Self {
            TIME_UNIT / BLIP_MAX_RATIO,
            size
        };

        blip
    }

    pub fn blip_set_rate(&mut self, clock_rate: f64, sample_rate: f64) {
        let factor = TIME_UNIT * sample_rate / clock_rate;
        self.factor = factor as u32;

        assert!(0 <= factor - self.factor && factor - self.factor < 1);

        if self.factor < factor {
            self.factor += 1;
        }
    }

    pub fn blip_clocks_needed(&self, samples: u32) -> u32 {
        let mut needed: u32;

        assert!(samples >= 0 && self.avail + samples <= self.size);

        needed = samples * TIME_UNIT;
        if needed < self.offset {
            return 0;
        }

        return (needed - self.offset + self.factor - 1) / self.factor;
    }

    pub fn blip_end_frame(&mut self, t: u32)
    {
        let off = t * self.factor + self.offset;
        self.avail += off >> TIME_BITS;
        self.offset = off & (TIME_UNIT - 1);

        assert!(self.avail <= self.size);
    }

    pub fn blip_clear(&mut self) {
        self.offset = self.factor / 2;
        self.avail = 0;
        self.integrator = 0;
        // memset( SAMPLES( m ), 0, (m->size + buf_extra) * sizeof (buf_t) );
    }

    pub fn blip_add_delta(&mut self, time: u32, mut delta: u32)
    {
        let fixed = time * self.factor + self.offset;
        let mut out: [i16];

        let phase_shift = FRAC_BITS - PHASE_BITS;
        let phase = fixed >> phase_shift & (PHASE_COUNT - 1);
        let in_step = BL_STEP[phase];
        let rev_step = BL_STEP[PHASE_COUNT - phase];

        let interp = fixed >> (phase_shift - DELTA_BITS) & (DELTA_UNIT - 1);
        let delta2 = (delta * interp) >> DELTA_BITS;
        delta -= delta2;

        // assert!(out <= )

        out[0] += in_step[0] * delta + in_step[HALF_WIDTH + 0] * delta2;
        out[1] += in_step[1] * delta + in_step[HALF_WIDTH + 1] * delta2;
        out[2] += in_step[2] * delta + in_step[HALF_WIDTH + 2] * delta2;
        out[3] += in_step[3] * delta + in_step[HALF_WIDTH + 3] * delta2;
        out[4] += in_step[4] * delta + in_step[HALF_WIDTH + 4] * delta2;
        out[5] += in_step[5] * delta + in_step[HALF_WIDTH + 5] * delta2;
        out[6] += in_step[6] * delta + in_step[HALF_WIDTH + 6] * delta2;
        out[7] += in_step[7] * delta + in_step[HALF_WIDTH + 7] * delta2;

        in_step = rev_step;
        out[0] += in_step[7] * delta + in_step[7 - HALF_WIDTH] * delta2;
        out[1] += in_step[6] * delta + in_step[6 - HALF_WIDTH] * delta2;
        out[2] += in_step[5] * delta + in_step[5 - HALF_WIDTH] * delta2;
        out[3] += in_step[4] * delta + in_step[4 - HALF_WIDTH] * delta2;
        out[4] += in_step[3] * delta + in_step[3 - HALF_WIDTH] * delta2;
        out[5] += in_step[2] * delta + in_step[2 - HALF_WIDTH] * delta2;
        out[6] += in_step[1] * delta + in_step[1 - HALF_WIDTH] * delta2;
        out[7] += in_step[0] * delta + in_step[0 - HALF_WIDTH] * delta2;
    }

    pub fn blip_add_delta_fast(&mut self, time: u32, mut delta: u32)
    {
        let fixed = time * self.factor + self.offset;
        let mut out: [i16];

        let interp = fixed >> (FRAC_BITS - DELTA_BITS) & (DELTA_UNIT - 1);
        let delta2 = delta * interp;

        out[7] += delta * DELTA_UNIT - delta2;
        out[8] += delta2;
    }
}