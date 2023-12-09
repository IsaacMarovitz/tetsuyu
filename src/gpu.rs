use bitflags::{bitflags, Flags};

pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

pub struct GPU {
    sy: u8,
    sx: u8,
    ly: u8,
    lyc: u8,
    wy: u8,
    wx: u8,
    lcdc: LCDC,
    lcds: LCDS,
    ram: [u8; 0x4000]
}

bitflags! {
    pub struct LCDC: u8 {
        // LCD & PPU enable: 0 = Off; 1 = On
        const LCD_ENABLE = 0b1000_0000;
        // Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const WINDOW_AREA = 0b0100_0000;
        // Window enable: 0 = Off; 1 = On
        const WINDOW_ENABLE  = 0b0010_0000;
        // BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
        const TILE_DATA_AREA = 0b0001_0000;
        // BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const TILE_MAP_AREA = 0b0000_1000;
        // OBJ size: 0 = 8×8; 1 = 8×16
        const OBJ_SIZE = 0b0000_01000;
        // OBJ enable: 0 = Off; 1 = On
        const OBJ_ENABLE = 0b0000_0010;
        // BG & Window enable (GB) / priority (CGB): 0 = Off; 1 = On
        const WINDOW_PRIORITY = 0b0000_0001;
    }
}

bitflags! {
    pub struct LCDS: u8 {
        // LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt.
        const LYC_SELECT = 0b0100_0000;
        // Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt.
        const MODE_2_SELECT = 0b0010_0000;
        // Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt.
        const MODE_1_SELECT = 0b0001_0000;
        // Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt.
        const MODE_0_SELECT = 0b0000_1000;
        // LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        const LYC_EQUALS = 0b0000_0100;
        // PPU mode (Read-only): Indicates the PPU’s current status.
    }
}
impl GPU {
    pub fn new() -> Self {
        Self {
            sy: 0,
            sx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            lcdc: LCDC::empty(),
            lcds: LCDS::empty(),
            ram: [0; 0x4000]
        }
    }

    pub fn read(&self, a: u16) -> u8 {
        match a {
            0xFF40 => self.lcdc.bits(),
            0xFF41 => self.lcds.bits(),
            0xFF42 => self.sy,
            0xFF43 => self.sx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0x00,
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF40 => self.lcdc = LCDC::from_bits(v).unwrap(),
            // TODO: Don't allow read-only bits to be set!
            0xFF41 => self.lcds = LCDS::from_bits(v).unwrap(),
            0xFF42 => self.sy = v,
            0xFF43 => self.sx = v,
            0xFF44 => print!("Attempted to write to LY!"),
            0xFF45 => self.lyc = v,
            0xFF4A => self.wy = v,
            0xFF4B => self.wx = v,
            _ => {},
        }
    }
}