pub struct BGPI {
    pub address: u8,
    pub auto_increment: bool
}

impl BGPI {
    pub fn new() -> Self {
        Self {
            address: 0,
            auto_increment: false
        }
    }

    pub fn read(&self) -> u8 {
        let a = if self.auto_increment {
            0x80
        } else {
            0x00
        };
        a | self.address
    }

    pub fn write(&mut self, v: u8) {
        self.auto_increment = v & 0x80 != 0x00;
        self.address = v & 0x3F;
    }
}
