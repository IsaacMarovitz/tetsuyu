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

    pub fn reload_if_zero(&mut self, max_length: u16) {
        if self.counter == 0 {
            self.counter = max_length;
        }
    }

    pub fn clear(&mut self) {
        self.enabled = false;
        self.counter = 0;
    }
}
