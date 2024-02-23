pub struct LengthCounter {
    pub enabled: bool,
    pub trigger: bool,
    pub counter: u16
}

impl LengthCounter {
    pub fn new() -> Self {
        Self {
            enabled: false,
            trigger: false,
            counter: 0
        }
    }

    pub fn cycle(&mut self) {
        if self.enabled && self.counter != 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.trigger = false;
            }
        }
    }

    pub fn reload(&mut self, length: u16) {
        if self.counter == 0 {
            self.counter = length;
        }
    }

    pub fn clear(&mut self) {
        self.enabled = false;
        self.trigger = false;
        self.counter = 0;
    }
}