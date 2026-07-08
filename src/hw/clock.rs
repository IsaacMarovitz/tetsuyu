pub struct Clock {
    dots: u64,
    speed_phase: u8,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            dots: 0,
            speed_phase: 0,
        }
    }

    pub fn tick(&mut self, double_speed: bool) -> bool {
        if double_speed {
            self.speed_phase ^= 1;
            if self.speed_phase == 0 {
                self.dots += 1;
                true
            } else {
                false
            }
        } else {
            self.dots += 1;
            true
        }
    }

    pub fn dots(&self) -> u64 {
        self.dots
    }
}
