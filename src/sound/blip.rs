use crate::blip::buffer::BlipBuf;

pub struct Blip {
    pub data: BlipBuf,
    pub from: u32,
    ampl: i32
}

impl Blip {
    pub fn new(data: BlipBuf) -> Self {
        Self {
            data,
            from: 0,
            ampl: 0
        }
    }

    pub(crate) fn set(&mut self, time: u32, ampl: i32) {
        self.from = time;
        let delta = ampl - self.ampl;
        self.ampl = ampl;
        self.data.add_delta(time, delta);
    }
}
