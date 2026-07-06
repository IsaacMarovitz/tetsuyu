pub struct VolumeEnvelope {
    pub volume: f32,
    pub period: u16,
    pub positive: bool,
    initial_volume: f32,
    period_counter: u16,
    enabled: bool,
}

impl VolumeEnvelope {
    pub fn new() -> Self {
        Self {
            volume: 0.0,
            period: 0,
            positive: false,
            initial_volume: 0.0,
            period_counter: 0,
            enabled: false,
        }
    }

    pub fn tick(&mut self) {
        if !self.enabled || self.period == 0 {
            return;
        }

        self.period_counter += 1;

        if self.period_counter >= self.period {
            self.period_counter = 0;

            if self.positive {
                if self.volume < 15.0 {
                    self.volume += 1.0;
                } else {
                    self.enabled = false;
                }
            } else {
                if self.volume > 0.0 {
                    self.volume -= 1.0;
                } else {
                    self.enabled = false;
                }
            }
        }
    }

    pub fn reload(&mut self) {
        self.volume = self.initial_volume;
        self.period_counter = 0;
        self.enabled = true;
    }

    pub fn read(&self) -> u8 {
        ((self.initial_volume as u8) << 4)
            | ((self.positive as u8) << 3)
            | (self.period as u8 & 0x07)
    }

    pub fn write(&mut self, v: u8) {
        // Zombie mode: a non-trigger NRx2 write adjusts the live volume
        // based on the OLD envelope state before the new config is stored.
        let mut vol = self.volume as i32;

        if self.period == 0 && self.enabled {
            vol += 1;
        } else if !self.positive {
            vol += 2;
        }

        let new_positive = (v & 0x08) != 0;
        if self.positive != new_positive {
            vol = 16 - vol;
        }

        self.volume = (vol & 0x0F) as f32;

        self.initial_volume = ((v & 0xF0) >> 4) as f32;
        self.positive = new_positive;
        self.period = (v & 0x07) as u16;
    }

    pub fn clear(&mut self) {
        self.volume = 0.0;
        self.period = 0;
        self.positive = false;
        self.initial_volume = 0.0;
        self.period_counter = 0;
        self.enabled = false;
    }
}
