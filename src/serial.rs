use std::io::Write;
use crate::memory::Memory;
use crate::mmu::Interrupts;

// TODO: Handle serial properly
pub struct Serial {
    pub interrupts: Interrupts,
    sb: u8,
    sc: u8
}

impl Serial {
    pub fn new() -> Self {
        Self {
            interrupts: Interrupts::empty(),
            sb: 0,
            sc: 0
        }
    }
}

impl Memory for Serial {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF01 => self.sb,
            0xFF02 => self.sc,
            _ => panic!("Read to unsupported Serial address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF01 => {
                self.sb = v;
                print!("{}", std::str::from_utf8(&[v]).unwrap());
                let _ = std::io::stdout().flush();
            },
            0xFF02 => self.sc = v,
            _ => panic!("Write to unsupported Serial address ({:#06x})!", a),
        }
    }
}