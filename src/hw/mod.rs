//! Cycle-accurate hardware core: peer chips on a shared bus, driven one
//! T-cycle at a time. The CPU is a micro-op sequencer that asserts the bus per
//! M-cycle; every other component is a `Chip` whose internal clock (`advance`)
//! and bus port (`bus`) are separate. The `Clock` owns time and the
//! double-speed divider; the `Motherboard` owns the chips and does arbitration,
//! routing, and DMA driving only.

pub mod apu;
pub mod bus;
pub mod clock;
pub mod cpu;
pub mod dma;
pub mod interrupt;
pub mod motherboard;
pub mod ppu;
pub mod sysbus;
pub mod timer;
