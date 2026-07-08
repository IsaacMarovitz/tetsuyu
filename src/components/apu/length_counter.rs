pub struct LengthCounter {
    pub enabled: bool,
    pub counter: u16,
}

impl LengthCounter {
    pub fn new() -> Self {
        Self {
            enabled: false,
            counter: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.enabled && self.counter > 0 {
            self.counter -= 1;
            if self.counter == 0 {
                return true;
            }
        }
        false
    }

    pub fn load(&mut self, value: u16, max_length: u16) {
        self.counter = max_length - value;
    }

    pub fn trigger_reload(&mut self, max_length: u16, extra: bool) {
        if self.counter == 0 {
            self.counter = max_length;
            if self.enabled && extra {
                self.counter -= 1;
            }
        }
    }

    // Extra clock applied when the counter is enabled (0->1 via NRx4) while the
    // next frame-sequencer step won't clock length. Returns true if this drove
    // the counter to zero, so the caller can disable the channel (unless the
    // same write also triggered).
    pub fn enable_clock(&mut self, was_enabled: bool, extra_phase: bool) -> bool {
        if !was_enabled && self.enabled && extra_phase && self.counter > 0 {
            self.counter -= 1;
            self.counter == 0
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.enabled = false;
        self.counter = 0;
    }
}
