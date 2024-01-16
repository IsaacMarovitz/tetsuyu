pub mod cpu;
pub mod joypad;
pub mod memory;
pub mod mmu;
pub mod mode;
pub mod ppu;
pub mod registers;
pub mod serial;
pub mod timer;

#[allow(unused_imports)]
pub mod prelude {
    pub use crate::components::{cpu::*, joypad::*, memory::*, mmu::*, mode::*, ppu::*, registers::*, serial::*, timer::*};
}