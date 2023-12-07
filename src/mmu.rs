pub struct MMU {
    rom: [u8; 0x8000],
    gpu: [u8; 0x2000],
    wram: [u8; 0x8000],
    hram: [u8; 0x7F],
    interrupt: u8,
    wram_bank: usize,
}

impl MMU {
    pub fn new(rom: [u8; 0x8000]) -> MMU {
        MMU {
            rom,
            gpu: [0; 0x2000],
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            interrupt: 0,
            wram_bank: 0x01
        }
    }

    pub fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x7FFF => self.rom[a as usize],
            0x8000..=0x9FFF => self.gpu[a as usize],
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000],
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank],
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000],
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank],
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80],
            0xFFFF => self.interrupt,
            _ => 0x00,
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x7FFF => self.rom[a as usize] = v,
            0x8000..=0x9FFF => self.gpu[a as usize] = v,
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000] = v,
            0xD000..=0xDFFF => self.wram[a as usize - 0xD000 + 0x1000 * self.wram_bank] = v,
            0xE000..=0xEFFF => self.wram[a as usize - 0xE000] = v,
            0xF000..=0xFDFF => self.wram[a as usize - 0xF000 + 0x1000 * self.wram_bank] = v,
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80] = v,
            0xFFFF => self.interrupt = v,
            _ => {},
        }
    }

    pub fn read_word(&self, a: u16) -> u16 {
        (self.read(a) as u16) | ((self.read(a + 1) as u16) << 8)
    }

    pub fn write_word(&mut self, a: u16, v: u16) {
        self.write(a, (v & 0xFF) as u8);
        self.write(a + 1, (v >> 8) as u8);
    }
}