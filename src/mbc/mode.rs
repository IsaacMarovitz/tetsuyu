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
            // All further type unimplemented
            CartTypes::MBC6 => MBCMode::Unsupported,
            CartTypes::MBC7SensorRumbleRamBat => MBCMode::Unsupported,
            CartTypes::PocketCamera => MBCMode::Unsupported,
            CartTypes::BandaiTAMA5 => MBCMode::Unsupported,
            CartTypes::HuC3 => MBCMode::Unsupported,
            CartTypes::HuC1RamBat => MBCMode::Unsupported,
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