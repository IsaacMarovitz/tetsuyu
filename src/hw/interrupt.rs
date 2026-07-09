use super::bus::{BusDir, Chip, Pins, Ticked};
use bitflags::bitflags;
use crate::components::prelude::io;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Default)]
    pub struct Interrupts: u8 {
        const JOYPAD  = 0b0001_0000;
        const SERIAL  = 0b0000_1000;
        const TIMER   = 0b0000_0100;
        const LCD     = 0b0000_0010;
        const V_BLANK = 0b0000_0001;
    }
}

impl Interrupts {
    /// Vector address for the highest-priority pending interrupt, and the bit
    /// to clear. Priority is V-Blank highest (lowest bit).
    pub fn highest(self) -> Option<(u16, Interrupts)> {
        for (bit, vector) in [
            (Interrupts::V_BLANK, 0x0040),
            (Interrupts::LCD, 0x0048),
            (Interrupts::TIMER, 0x0050),
            (Interrupts::SERIAL, 0x0058),
            (Interrupts::JOYPAD, 0x0060),
        ] {
            if self.contains(bit) {
                return Some((vector, bit));
            }
        }
        None
    }
}

pub struct InterruptController {
    iflag: Interrupts,
    ienable: Interrupts,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            iflag: Interrupts::empty(),
            ienable: Interrupts::empty(),
        }
    }

    pub fn request(&mut self, irq: Interrupts) {
        self.iflag |= irq;
    }

    /// Bits that are both requested and enabled — the CPU's IRQ input.
    pub fn pending(&self) -> Interrupts {
        self.iflag & self.ienable
    }

    pub fn acknowledge(&mut self, bit: Interrupts) {
        self.iflag &= !bit;
    }
}

impl Chip for InterruptController {
    fn bus(&mut self, pins: &mut Pins) -> Ticked {
        // IF/IE are pure registers; nothing free-running to advance.
        match pins.address {
            io::IF if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = self.iflag.bits() | 0xE0,
                BusDir::Write => self.iflag = Interrupts::from_bits_truncate(pins.data),
                BusDir::Idle => {}
            },
            0xFFFF if pins.selected(true) => match pins.dir {
                BusDir::Read => pins.data = self.ienable.bits(),
                BusDir::Write => self.ienable = Interrupts::from_bits_truncate(pins.data),
                BusDir::Idle => {}
            },
            _ => {}
        }
        Ticked::default()
    }
}
