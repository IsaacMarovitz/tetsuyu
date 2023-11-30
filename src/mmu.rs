pub struct MMU {
    rom: [u8; 0x8000],
    wram: [u8; 0x8000],
    hram: [u8; 0x7f],
    interrupt: u8,
}

impl MMU {
    pub fn new(rom: [u8; 0x8000]) -> MMU {
        MMU {
            rom,
            wram: [0; 0x8000],
            hram: [0; 0x7f],
            interrupt: 0,
        }
    }

    pub fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x8000 => self.rom[a as usize],
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000],
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80],
            0xFFFF => self.interrupt,
            _ => 0x00,
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0xC000..=0xCFFF => self.wram[a as usize - 0xC000] = v,
            0xFF80..=0xFFFE => self.hram[a as usize - 0xFF80] = v,
            0xFFFF => self.interrupt = v,
            _ => {},
        }
    }
}