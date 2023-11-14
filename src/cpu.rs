use crate::mode::GBMode;
use crate::registers::Registers;

pub struct CPU {
    reg: Registers
}

impl CPU {
    pub fn new(mode: GBMode) -> CPU {
        CPU {
            reg: Registers::new(mode)
        }
    }
}