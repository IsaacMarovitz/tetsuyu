pub struct VolumeEnvelope {
    pub volume: u8,
    pub period: u16,
    pub positive: bool,
}

impl VolumeEnvelope {
    pub fn new() -> Self {
        Self {
            volume: 0,
            period: 0,
            positive: false
        }
    }

    pub fn cycle(&mut self) {
        if self.period == 0 {
            return;
        }

        let volume = if self.positive {
            self.volume.wrapping_add(1)
        } else {
            self.volume.wrapping_sub(1)
        };

        if volume <= 15 {
            self.volume = volume;
        }
    }

    pub fn reload() {
        // TODO!!!
    }

    pub fn clear(&mut self) {
        self.volume = 0;
        self.period = 0;
        self.positive = false;
    }
}