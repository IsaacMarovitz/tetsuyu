use bitflags::bitflags;

#[derive(PartialEq, Copy, Clone)]
pub enum Priority {
    Color0,
    Priority,
    Normal,
}

#[derive(PartialEq, Copy, Clone)]
pub enum PPUMode {
    OAMScan = 2,
    Draw = 3,
    HBlank = 0,
    VBlank = 1,
}

bitflags! {
    #[derive(PartialEq, Copy, Clone)]
    pub struct Attributes: u8 {
        const PRIORITY     = 0b1000_0000;
        const Y_FLIP       = 0b0100_0000;
        const X_FLIP       = 0b0010_0000;
        const PALETTE_NO_0 = 0b0001_0000;
        const BANK         = 0b0000_1000;
    }
}

bitflags! {
    #[derive(PartialEq, Copy, Clone)]
    pub struct LCDC: u8 {
        // LCD & PPU enable: 0 = Off; 1 = On
        const LCD_ENABLE      = 0b1000_0000;
        // Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const WINDOW_AREA     = 0b0100_0000;
        // Window enable: 0 = Off; 1 = On
        const WINDOW_ENABLE   = 0b0010_0000;
        // BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
        const TILE_DATA_AREA  = 0b0001_0000;
        // BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
        const BG_TILE_MAP_AREA   = 0b0000_1000;
        // OBJ size: 0 = 8×8; 1 = 8×16
        const OBJ_SIZE        = 0b0000_0100;
        // OBJ enable: 0 = Off; 1 = On
        const OBJ_ENABLE      = 0b0000_0010;
        // BG & Window enable (GB) / priority (CGB): 0 = Off; 1 = On
        const WINDOW_PRIORITY = 0b0000_0001;
    }
}

bitflags! {
    #[derive(PartialEq, Copy, Clone)]
    pub struct LCDS: u8 {
        // LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt.
        const LYC_SELECT    = 0b0100_0000;
        // Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt.
        const MODE_2_SELECT = 0b0010_0000;
        // Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt.
        const MODE_1_SELECT = 0b0001_0000;
        // Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt.
        const MODE_0_SELECT = 0b0000_1000;
        // LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
        const LYC_EQUALS    = 0b0000_0100;
        // PPU mode (Read-only): Indicates the PPU’s current status.
    }
}
