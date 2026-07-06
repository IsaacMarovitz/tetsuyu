use crate::hw::interrupt::Interrupts;
use crate::components::prelude::*;
use std::io::Write;

// TODO: Handle serial properly
pub struct Serial {
    pub interrupts: Interrupts,
    sb: u8,
    sc: u8,
    output: Vec<u8>,
    print: bool,
    mode: GBMode
}

impl Serial {
    pub fn new(print: bool, mode: GBMode) -> Self {
        Self {
            interrupts: Interrupts::empty(),
            sb: 0,
            sc: 0,
            output: Vec::new(),
            print,
            mode
        }
    }

    /// Bytes transmitted so far (captured when a transfer is started).
    pub fn output(&self) -> &[u8] {
        &self.output
    }
}

impl Memory for Serial {
    fn read(&self, a: u16) -> u8 {
        match a {
            0xFF01 => self.sb,
            0xFF02 => {
                let mask = if self.mode == GBMode::DMG { 0x7E } else { 0x7C };
                mask | self.sc
            }
            _ => panic!("Read to unsupported Serial address ({:#06x})!", a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0xFF01 => self.sb = v,
            0xFF02 => {
                self.sc = v;
                // Bit 7 starts a transfer; the byte staged in SB is what is sent.
                if v & 0x80 != 0 {
                    self.output.push(self.sb);
                    if self.print {
                        print!("{}", self.sb as char);
                        let _ = std::io::stdout().flush();
                    }
                }
            }
            _ => panic!("Write to unsupported Serial address ({:#06x})!", a),
        }
    }
}
