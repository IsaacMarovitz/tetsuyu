use super::interrupt::Interrupts;

/// Direction the current bus master is driving this M-cycle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BusDir {
    /// No transfer this cycle (internal/idle M-cycle).
    Idle,
    Read,
    Write,
}

/// Which chip is driving the address/data traces this M-cycle. During OAM DMA
/// the DMA engine seizes the bus; a CPU access that collides sees open bus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BusMaster {
    Cpu,
    OamDma,
}

/// The shared traces. Exactly one master drives them per M-cycle; every chip
/// watches them and asserts data / latches writes only on the transfer dot.
pub struct Pins {
    pub address: u16,
    pub data: u8,
    pub dir: BusDir,
    pub master: BusMaster,
    /// True only on the dot the transfer commits (last dot of the access
    /// M-cycle). Chips advance their internal state every dot but only touch
    /// the bus when this is set — mirroring data latching at the M-cycle edge.
    pub transfer: bool,
}

impl Pins {
    pub fn new() -> Self {
        Self {
            address: 0,
            data: 0xFF,
            dir: BusDir::Idle,
            master: BusMaster::Cpu,
            transfer: false,
        }
    }

    /// A chip calls this in its `tick` to decide whether to respond this dot.
    pub fn selected(&self, in_range: bool) -> bool {
        self.transfer && in_range
    }
}

/// What a chip produced on a single dot. The motherboard routes these; chips
/// never reach into one another.
#[derive(Default, Clone, Copy)]
pub struct Ticked {
    pub irq: Interrupts,
    /// PPU entered HBlank on this dot (routes to HDMA). Only the PPU sets it.
    pub hblank_edge: bool,
}

impl Ticked {
    pub fn merge(&mut self, other: Ticked) {
        self.irq |= other.irq;
        self.hblank_edge |= other.hblank_edge;
    }
}

pub trait Chip {
    fn advance(&mut self, _base_dot: bool) -> Ticked {
        Ticked::default()
    }
    fn bus(&mut self, _pins: &mut Pins) -> Ticked {
        Ticked::default()
    }
}
