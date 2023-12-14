pub trait Memory {
    fn read(&self, a: u16) -> u8;
    fn write(&mut self, a: u16, v: u8);

    fn read_word(&self, a: u16) -> u16 {
        (self.read(a) as u16) | ((self.read(a + 1) as u16) << 8)
    }
    fn write_word(&mut self, a: u16, v: u16) {
        self.write(a, (v & 0xFF) as u8);
        self.write(a + 1, (v >> 8) as u8);
    }
}