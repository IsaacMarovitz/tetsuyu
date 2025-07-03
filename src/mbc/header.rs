use crate::mbc::licensee::Licensee;
use crate::mbc::mode::CartTypes;
use num_traits::FromPrimitive;
use std::fmt;
use std::fmt::Formatter;

#[derive(Clone)]
pub struct Header {
    pub cart_type: CartTypes,
    pub licensee: Licensee,
    pub destination: Destination,
    pub rom_size: u8,
    pub ram_size: u8,
    pub title: String,
    pub manufacturer_code: String,
    pub cgb_flag: CGBFlag,
    pub sgb_flag: bool,
}

impl Header {
    pub fn new(buffer: Vec<u8>) -> Self {
        let cart_type: CartTypes =
            FromPrimitive::from_u8(buffer[0x0147]).expect("Failed to get cart type!");
        let licensee = match Licensee::old_licensee(buffer[0x014B]) {
            Some(code) => code,
            None => {
                let string = String::from_utf8_lossy(&buffer[0x0144..=0x0145]);
                let str = string.to_owned();
                let code = Licensee::new_licensee(&str);
                code
            }
        };
        let destination =
            FromPrimitive::from_u8(buffer[0x014A]).expect("Failed to get cart destination!");

        let rom_size = buffer[0x0148];
        let ram_size = buffer[0x0149];

        let title = String::from_utf8_lossy(&buffer[0x0134..=0x013E]).to_string();
        let manufacturer_code = String::from_utf8_lossy(&buffer[0x013F..=0x0142]).to_string();

        let cgb_flag = match FromPrimitive::from_u8(buffer[0x0143]) {
            Some(flag) => flag,
            None => CGBFlag::DMGOnly,
        };
        let sgb_flag = buffer[0x0146] == 0x03;

        Self {
            cart_type,
            licensee,
            destination,
            rom_size,
            ram_size,
            title,
            manufacturer_code,
            cgb_flag,
            sgb_flag,
        }
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} (Licensee: {}, Cart Type: {})",
            self.title, self.licensee, self.cart_type
        )?;
        writeln!(
            f,
            "ROM: {}, RAM: {}, CGB: {:?}, SGB: {}",
            self.rom_size, self.ram_size, self.cgb_flag, self.sgb_flag
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, FromPrimitive, Debug)]
pub enum Destination {
    Japan = 0x00,
    Oveseas = 0x01,
}

#[derive(Clone, Copy, PartialEq, FromPrimitive, Debug)]
pub enum CGBFlag {
    BackwardsCompatible = 0x80,
    CGBOnly = 0xC0,
    DMGOnly,
}
