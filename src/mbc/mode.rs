use std::fmt;
use std::fmt::{Formatter};

#[derive(Clone, Copy, PartialEq, FromPrimitive, Debug)]
pub enum CartTypes {
    RomOnly = 0x00,
    MBC1 = 0x01,
    MBC1Ram = 0x02,
    MBC1RamBat = 0x03,
    MBC2 = 0x05,
    MBC2Bat = 0x06,
    // Unused
    RomRam = 0x08,
    // Unused
    RomRamBat = 0x09,
    MMM01 = 0x0B,
    MMM01Ram = 0x0C,
    MMM01RamBat = 0x0D,
    MBC3TimerBat = 0x0F,
    MBC3TimerRamBat = 0x10,
    MBC3 = 0x11,
    MBC3Ram = 0x12,
    MBC3RamBat = 0x13,
    MBC5 = 0x19,
    MBC5Ram = 0x1A,
    MBC5RamBat = 0x1B,
    MBC5Rumble = 0x1C,
    MBC5RumbleRam = 0x1D,
    MBC5RumbleRamBat = 0x1E,
    MBC6 = 0x20,
    MBC7SensorRumbleRamBat = 0x22,
    PocketCamera = 0xFC,
    BandaiTAMA5 = 0xFD,
    HuC3 = 0xFE,
    HuC1RamBat = 0xFF
}

impl CartTypes {
    pub fn get_mbc(&self) -> MBCMode {
        match self {
            CartTypes::RomOnly => MBCMode::RomOnly,
            CartTypes::MBC1 => MBCMode::MBC1,
            CartTypes::MBC1Ram => MBCMode::MBC1,
            CartTypes::MBC1RamBat => MBCMode::MBC1,
            CartTypes::MBC2 => MBCMode::MBC2,
            CartTypes::MBC2Bat => MBCMode::MBC2,
            CartTypes::RomRam => MBCMode::RomOnly,
            CartTypes::RomRamBat => MBCMode::RomOnly,
            CartTypes::MMM01 => MBCMode::RomOnly,
            CartTypes::MMM01Ram => MBCMode::RomOnly,
            CartTypes::MMM01RamBat => MBCMode::RomOnly,
            CartTypes::MBC3TimerBat => MBCMode::MBC3,
            CartTypes::MBC3TimerRamBat => MBCMode::MBC3,
            CartTypes::MBC3 => MBCMode::MBC3,
            CartTypes::MBC3Ram => MBCMode::MBC3,
            CartTypes::MBC3RamBat => MBCMode::MBC3,
            CartTypes::MBC5 => MBCMode::MBC5,
            CartTypes::MBC5Ram => MBCMode::MBC5,
            CartTypes::MBC5RamBat => MBCMode::MBC5,
            CartTypes::MBC5Rumble => MBCMode::MBC5,
            CartTypes::MBC5RumbleRam => MBCMode::MBC5,
            CartTypes::MBC5RumbleRamBat => MBCMode::MBC5,
            // All further types unimplemented
            CartTypes::MBC6 => MBCMode::Unsupported,
            CartTypes::MBC7SensorRumbleRamBat => MBCMode::Unsupported,
            CartTypes::PocketCamera => MBCMode::Unsupported,
            CartTypes::BandaiTAMA5 => MBCMode::Unsupported,
            CartTypes::HuC3 => MBCMode::Unsupported,
            CartTypes::HuC1RamBat => MBCMode::Unsupported,
        }
    }
}

impl fmt::Display for CartTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CartTypes::RomOnly => write!(f, "ROM ONLY"),
            CartTypes::MBC1 => write!(f, "MBC1"),
            CartTypes::MBC1Ram => write!(f, "MBC1+RAM"),
            CartTypes::MBC1RamBat => write!(f, "MBC1+RAM+BATTERY"),
            CartTypes::MBC2 => write!(f, "MBC2"),
            CartTypes::MBC2Bat => write!(f, "MBC2+BATTERY"),
            CartTypes::RomRam => write!(f, "ROM+RAM"),
            CartTypes::RomRamBat => write!(f, "ROM+RAM+BATTERY"),
            CartTypes::MMM01 => write!(f, "MMM01"),
            CartTypes::MMM01Ram => write!(f, "MMM01+RAM"),
            CartTypes::MMM01RamBat => write!(f, "MMM01+RAM+BATTERY"),
            CartTypes::MBC3TimerBat => write!(f, "MBC3+TIMER+BATTERY"),
            CartTypes::MBC3TimerRamBat => write!(f, "MBC3+TIMER+RAM+BATTERY"),
            CartTypes::MBC3 => write!(f, "MBC3"),
            CartTypes::MBC3Ram => write!(f, "MBC3+RAM"),
            CartTypes::MBC3RamBat => write!(f, "MBC3+RAM+BATTERY"),
            CartTypes::MBC5 => write!(f, "MBC5"),
            CartTypes::MBC5Ram => write!(f, "MBC5+RAM"),
            CartTypes::MBC5RamBat => write!(f, "MBC5+RAM+BATTERY"),
            CartTypes::MBC5Rumble => write!(f, "MBC5+RUMBLE"),
            CartTypes::MBC5RumbleRam => write!(f, "MBC5+RUMBLE+RAM"),
            CartTypes::MBC5RumbleRamBat => write!(f, "MBC5+RUMBLE+RAM+BATTERY"),
            CartTypes::MBC6 => write!(f, "MBC6"),
            CartTypes::MBC7SensorRumbleRamBat => write!(f, "MBC7+SENSOR+RUMBLE+RAM+BATTERY"),
            CartTypes::PocketCamera => write!(f, "POCKET CAMERA"),
            CartTypes::BandaiTAMA5 => write!(f, "BANDAI TAMA5"),
            CartTypes::HuC3 => write!(f, "HuC3"),
            CartTypes::HuC1RamBat => write!(f, "HuC1+RAM+BATTERY"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MBCMode {
    RomOnly,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
    Unsupported
}

impl fmt::Display for MBCMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MBCMode::RomOnly => write!(f, "ROM Only"),
            MBCMode::MBC1 => write!(f, "MBC1"),
            MBCMode::MBC2 => write!(f, "MBC2"),
            MBCMode::MBC3 => write!(f, "MBC3"),
            MBCMode::MBC5 => write!(f, "MBC5"),
            MBCMode::Unsupported => write!(f, "Unsupported"),
        }
    }
}