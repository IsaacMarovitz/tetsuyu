pub struct VolumeEnvelope {
    pub volume: f32,
    pub period: u16,
    pub positive: bool,
    initial_volume: f32,
    period_counter: u16,
}

impl VolumeEnvelope {
    pub fn new() -> Self {
        Self {
            volume: 0.0,
            period: 0,
            positive: false,
            initial_volume: 0.0,
            period_counter: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.period == 0 {
            return;
        }

        self.period_counter += 1;

        if self.period_counter >= self.period {
            self.period_counter = 0;

            if self.positive {
                if self.volume < 15.0 {
                    self.volume += 1.0;
                }
            } else {
                if self.volume > 0.0 {
                    self.volume -= 1.0;
                }
            }
        }
    }

    pub fn reload(&mut self) {
        self.volume = self.initial_volume;
        self.period_counter = 0;
    }

    pub fn set_initial_volume(&mut self, vol: f32) {
        self.initial_volume = vol;
        self.volume = vol;
    }

    pub fn clear(&mut self) {
        self.volume = 0.0;
        self.period = 0;
        self.positive = false;
        self.initial_volume = 0.0;
        self.period_counter = 0;
    }
}
