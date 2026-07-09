use crate::components::mode::GBMode;
use bitflags::bitflags;
use std::fmt;
use std::fmt::Formatter;

pub mod io {
    /* Joypad and Serial */
    /// Joypad (R/W)
    pub const JOYP: u16 = 0xFF00;
    /// Serial transfer data (R/W)
    pub const SB: u16 = 0xFF01;
    /// Serial Transfer Control (R/W)
    pub const SC: u16 = 0xFF02;

    /* Missing */

    /* Timers */
    /// Divider Register (R/W)
    pub const DIV: u16 = 0xFF04;
    /// Timer counter (R/W)
    pub const TIMA: u16 = 0xFF05;
    /// Timer Modulo (R/W)
    pub const TMA: u16 = 0xFF06;
    /// Timer Control (R/W)
    pub const TAC: u16 = 0xFF07;

    /* Missing */

    /// Interrupt Flag (R/W)
    pub const IF: u16 = 0xFF0F;

    /* Sound */
    /// Channel 1 Sweep register (R/W)
    pub const NR10: u16 = 0xFF10;
    /// Channel 1 Sound length/Wave pattern duty (R/W)
    pub const NR11: u16 = 0xFF11;
    /// Channel 1 Volume Envelope (R/W)
    pub const NR12: u16 = 0xFF12;
    /// Channel 1 Frequency lo (Write Only)
    pub const NR13: u16 = 0xFF13;
    /// Channel 1 Frequency hi (R/W)
    pub const NR14: u16 = 0xFF14;
    /* NR20 does not exist */
    /// Channel 2 Sound Length/Wave Pattern Duty (R/W)
    pub const NR21: u16 = 0xFF16;
    /// Channel 2 Volume Envelope (R/W)
    pub const NR22: u16 = 0xFF17;
    /// Channel 2 Frequency lo data (W)
    pub const NR23: u16 = 0xFF18;
    /// Channel 2 Frequency hi data (R/W)
    pub const NR24: u16 = 0xFF19;
    /// Channel 3 Sound on/off (R/W)
    pub const NR30: u16 = 0xFF1A;
    /// Channel 3 Sound Length
    pub const NR31: u16 = 0xFF1B;
    /// Channel 3 Select output level (R/W)
    pub const NR32: u16 = 0xFF1C;
    /// Channel 3 Frequency's lower data (W)
    pub const NR33: u16 = 0xFF1D;
    /// Channel 3 Frequency's higher data (R/W)
    pub const NR34: u16 = 0xFF1E;
    /* NR40 does not exist */
    /// Channel 4 Sound Length (R/W)
    pub const NR41: u16 = 0xFF20;
    /// Channel 4 Volume Envelope (R/W)
    pub const NR42: u16 = 0xFF21;
    /// Channel 4 Polynomial Counter (R/W)
    pub const NR43: u16 = 0xFF22;
    /// Channel 4 Counter/consecutive, Initial (R/W)
    pub const NR44: u16 = 0xFF23;
    /// Channel control / ON-OFF / Volume (R/W)
    pub const NR50: u16 = 0xFF24;
    /// Selection of Sound output terminal (R/W)
    pub const NR51: u16 = 0xFF25;
    /// Sound on/off
    pub const NR52: u16 = 0xFF26;

    /* Missing */

    /// Wave pattern start
    pub const WAV_START: u16 = 0xFF30;
    /// Wave pattern end
    pub const WAV_END: u16 = 0xFF3F;

    /// LCD Control (R/W)
    pub const LCDC: u16 = 0xFF40;
    /// LCDC Status (R/W)
    pub const STAT: u16 = 0xFF41;
    /// Scroll Y (R/W)
    pub const SCY: u16 = 0xFF42;
    /// Scroll X (R/W)
    pub const SCX: u16 = 0xFF43;
    /// LCDC Y-Coordinate (R)
    pub const LY: u16 = 0xFF44;
    /// LY Compare (R/W)
    pub const LYC: u16 = 0xFF45;
    /// DMA Transfer and Start Address (W)
    pub const DMA: u16 = 0xFF46;
    /// BG Palette Data (R/W) - Non CGB Mode Only
    pub const BGP: u16 = 0xFF47;
    /// Object Palette 0 Data (R/W) - Non CGB Mode Only
    pub const OBP0: u16 = 0xFF48;
    /// Object Palette 1 Data (R/W) - Non CGB Mode Only
    pub const OBP1: u16 = 0xFF49;
    /// Window Y Position (R/W)
    pub const WY: u16 = 0xFF4A;
    /// Window X Position minus 7 (R/W)
    pub const WX: u16 = 0xFF4B;
    /// Controls DMG mode and PGB mode
    pub const KEY0: u16 = 0xFF4C;
    /// CGB Mode Only - Prepare Speed Switch
    pub const KEY1: u16 = 0xFF4D;

    /* Missing */

    /// CGB Mode Only - VRAM Bank
    pub const VBK: u16 = 0xFF4F;
    /// Write to disable the boot ROM mapping
    pub const BANK: u16 = 0xFF50;

    /* CGB DMA */
    /// CGB Mode Only - New DMA Source, High
    pub const HDMA1: u16 = 0xFF51;
    /// CGB Mode Only - New DMA Source, Low
    pub const HDMA2: u16 = 0xFF52;
    /// CGB Mode Only - New DMA Destination, High
    pub const HDMA3: u16 = 0xFF53;
    /// CGB Mode Only - New DMA Destination, Low
    pub const HDMA4: u16 = 0xFF54;
    /// CGB Mode Only - New DMA Length/Mode/Start
    pub const HDMA5: u16 = 0xFF55;

    /* IR */
    /// CGB Mode Only - Infrared Communications Port
    pub const RP: u16 = 0xFF56;

    /* Missing */

    /* CGB Palettes */
    /// CGB Mode Only - Background Palette Index
    pub const BGPI: u16 = 0xFF68;
    /// CGB Mode Only - Background Palette Data
    pub const BGPD: u16 = 0xFF69;
    /// CGB Mode Only - Object Palette Index
    pub const OBPI: u16 = 0xFF6A;
    /// CGB Mode Only - Object Palette Data
    pub const OBPD: u16 = 0xFF6B;
    /// Affects object priority (X based or index based)
    pub const OPRI: u16 = 0xFF6C;

    /* Missing */

    /// CGB Mode Only - WRAM Bank
    pub const SVBK: u16 = 0xFF70;
    /// Palette Selection Mode, controls the PSW and key combo
    pub const PSM: u16 = 0xFF71;
    /// X position of the palette switching window
    pub const PSWX: u16 = 0xFF72;
    /// Y position of the palette switching window
    pub const PSWY: u16 = 0xFF73;
    /// Key combo to trigger the palette switching window
    pub const PSW: u16 = 0xFF74;
    /// Bits 0-2 control PHI, A15 and ¬CS, respectively.  Bits 4-6 control the I/O directions of bits 0-2 (0 is R, 1 is W)
    pub const PGB: u16 = 0xFF75;
    /// Channels 1 and 2 amplitudes
    pub const PCM12: u16 = 0xFF76;
    /// Channels 3 and 4 amplitudes
    pub const PCM34: u16 = 0xFF77;
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
}

impl PartialEq for Registers {
    fn eq(&self, other: &Registers) -> bool {
        self.a == other.a
            && self.f == other.f
            && self.b == other.b
            && self.c == other.c
            && self.d == other.d
            && self.e == other.e
            && self.h == other.h
            && self.l == other.l
            && self.pc == other.pc
            && self.sp == other.sp
    }
}

bitflags! {
    pub struct Flags: u8 {
        // Carry Flag
        const C = 0b0001_0000;
        // Half-Carry Flag
        const H = 0b0010_0000;
        // Subtract Flag
        const N = 0b0100_0000;
        // Zero Flag
        const Z = 0b1000_0000;
    }
}

impl Registers {
    // Registers can be paired to be used for 16-bit operations
    // A+F, B+C, D+E, H+L
    pub fn get_af(&self) -> u16 {
        (u16::from(self.a) << 8) | u16::from(self.f)
    }

    pub fn get_bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    pub fn get_de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    pub fn get_hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
    }

    pub fn set_af(&mut self, x: u16) {
        self.a = (x >> 8) as u8;
        self.f = (x & 0x00F0) as u8;
    }

    pub fn set_bc(&mut self, x: u16) {
        self.b = (x >> 8) as u8;
        self.c = (x & 0x00FF) as u8;
    }

    pub fn set_de(&mut self, x: u16) {
        self.d = (x >> 8) as u8;
        self.e = (x & 0x00FF) as u8;
    }

    pub fn set_hl(&mut self, x: u16) {
        self.h = (x >> 8) as u8;
        self.l = (x & 0x00FF) as u8;
    }

    pub fn get_flag(&self, flag: Flags) -> bool {
        Flags::from_bits(self.f).unwrap().contains(flag)
    }

    pub fn set_flag(&mut self, flag: Flags, state: bool) {
        if state {
            self.f |= flag.bits();
        } else {
            self.f &= !flag.bits();
        }
    }

    pub fn new(mode: GBMode) -> Registers {
        match mode {
            GBMode::DMG => Registers {
                a: 0x01,
                f: (Flags::C | Flags::H | Flags::Z).bits(),
                b: 0x00,
                c: 0x13,
                d: 0x00,
                e: 0xD8,
                h: 0x01,
                l: 0x4D,
                pc: 0x0000,
                sp: 0xFFFE,
            },
            GBMode::CGB => Registers {
                a: 0x11,
                f: (Flags::Z).bits(),
                b: 0x00,
                c: 0x00,
                d: 0xFF,
                e: 0x56,
                h: 0x00,
                l: 0x0D,
                pc: 0x0000,
                sp: 0xFFFE,
            },
        }
    }
}

impl fmt::Display for Registers {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[A: {:#02x}, F: {:#02x}, B: {:#02x}, C: {:#02x}, D: {:#02x}, E: {:#02x}, H: {:#02x}, L: {:#02x}, PC: {:#04x}, SP: {:#04x}]",
            self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l, self.pc, self.sp
        )
    }
}
