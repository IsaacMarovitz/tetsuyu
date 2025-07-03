pub struct VolumeEnvelope {
    pub volume: f32,
    pub period: u16,
    pub positive: bool,
}

impl VolumeEnvelope {
    pub fn new() -> Self {
        Self {
            volume: 0f32,
            period: 0,
            positive: false,
        }
    }

    pub fn cycle(&mut self) {
        if self.period == 0 {
            return;
        }

        let volume = if self.positive {
            // self.volume.wrapping_add(1)
            0f32
        } else {
            //println!("Decreasing");
            self.volume + 0.01f32
        };

        if volume <= 15f32 {
            self.volume = volume;
        }
    }

    pub fn reload() {
        // TODO!!!
    }

    pub fn clear(&mut self) {
        self.volume = 0f32;
        self.period = 0;
        self.positive = false;
    }
}
