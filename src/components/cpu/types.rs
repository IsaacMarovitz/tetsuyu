#[derive(Clone, Copy, PartialEq)]
pub enum R8 {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
}

#[derive(Clone, Copy)]
pub enum R16 {
    Bc,
    De,
    Hl,
    Sp,
}

#[derive(Clone, Copy)]
pub enum R16Stk {
    Bc,
    De,
    Hl,
    Af,
}

#[derive(Clone, Copy)]
pub enum Cond {
    Nz,
    Z,
    Nc,
    C,
}

#[derive(Clone, Copy)]
pub enum AluOp {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
}

#[derive(Clone, Copy)]
pub enum RotOp {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Swap,
    Srl,
}

#[derive(Clone, Copy)]
pub enum Addr {
    Bc,
    De,
    Hl,
    HighZ,
    HighC,
    Wz,
    Sp,
}

#[derive(Clone, Copy)]
pub enum Byte {
    Reg(R8),
    Z,
    PcHigh,
    PcLow,
    SpHigh,
    SpLow,
    StkHigh(R16Stk),
    StkLow(R16Stk),
}

pub fn r8_of(idx: u8) -> R8 {
    match idx {
        0 => R8::B,
        1 => R8::C,
        2 => R8::D,
        3 => R8::E,
        4 => R8::H,
        5 => R8::L,
        7 => R8::A,
        _ => unreachable!("index 6 is (HL), not a register"),
    }
}

pub fn rp_of(p: u8) -> R16 {
    match p {
        0 => R16::Bc,
        1 => R16::De,
        2 => R16::Hl,
        _ => R16::Sp,
    }
}

pub fn rp2_of(p: u8) -> R16Stk {
    match p {
        0 => R16Stk::Bc,
        1 => R16Stk::De,
        2 => R16Stk::Hl,
        _ => R16Stk::Af,
    }
}

pub fn cc_of(y: u8) -> Cond {
    match y {
        0 => Cond::Nz,
        1 => Cond::Z,
        2 => Cond::Nc,
        _ => Cond::C,
    }
}

pub fn alu_of(y: u8) -> AluOp {
    match y {
        0 => AluOp::Add,
        1 => AluOp::Adc,
        2 => AluOp::Sub,
        3 => AluOp::Sbc,
        4 => AluOp::And,
        5 => AluOp::Xor,
        6 => AluOp::Or,
        _ => AluOp::Cp,
    }
}

pub fn rot_of(y: u8) -> RotOp {
    match y {
        0 => RotOp::Rlc,
        1 => RotOp::Rrc,
        2 => RotOp::Rl,
        3 => RotOp::Rr,
        4 => RotOp::Sla,
        5 => RotOp::Sra,
        6 => RotOp::Swap,
        _ => RotOp::Srl,
    }
}

pub enum MicroOp {
    Fetch,
    ImmZ,
    ImmW,
    LoadZ(Addr),
    LoadW(Addr),
    Store(Addr, Byte),
    Internal,
    Exec(Effect),
}

#[derive(Clone, Copy)]
pub enum Effect {
    LdR(R8, R8),
    LdRZ(R8),
    Alu(AluOp, R8),
    AluZ(AluOp),
    IncR(R8),
    DecR(R8),
    IncZ,
    DecZ,
    Inc16(R16),
    Dec16(R16),
    /// 16-bit increment/decrement that raises no OAM glitch of its own. Used by
    /// `LD A,(HL+/-)`, where the inc/dec shares its M-cycle with the memory read
    /// and is absorbed into a single read-during-increase corruption.
    Inc16NoGlitch(R16),
    Dec16NoGlitch(R16),
    AddHl(R16),
    AccRot(RotOp),
    Rot(RotOp, R8),
    RotZ(RotOp),
    Bit(u8, R8),
    BitZ(u8),
    Res(u8, R8),
    ResZ(u8),
    Set(u8, R8),
    SetZ(u8),
    Daa,
    Cpl,
    Scf,
    Ccf,
    LdRpWz(R16),
    SetStkWz(R16Stk),
    SetPcWz,
    SetPcHl,
    SetPc(u16),
    /// Set PC to the vector latched at the start of the low-byte push cycle.
    SetPcIsr,
    /// Arm the ISR vector latch: run as a free act at the start of the low-byte
    /// push M-cycle, it flags the motherboard to sample live `IE & IF` now.
    IsrArmLatch,
    JrZ,
    SpDec,
    /// SP decrement that does *not* raise an OAM glitch. Used for a push's
    /// second `dec sp`, whose glitched write collapses into the store sharing
    /// its M-cycle ("push triggers 4 times but behaves like 3 writes").
    SpDecNoGlitch,
    SpInc,
    /// Free act raising the DMG "read-during-increase" OAM glitch for the given
    /// address (the low-byte read of a stack pop, which shares its M-cycle with
    /// the first SP increment). Does not modify any register.
    OamReadInc(Addr),
    /// Free act raising a plain OAM "read" glitch for the given address (the
    /// high-byte read of a stack pop). Does not modify any register.
    OamRead(Addr),
    /// Free act raising an OAM "write" glitch for the given address (an actual
    /// store into OAM, e.g. a push). Does not modify any register.
    OamWrite(Addr),
    WzInc,
    LdSpHl,
    AddSpE,
    LdHlSpE,
    Ei,
    Di,
    Reti,
    Halt,
    Stop,
    DecodeCb,
    BranchRel(Cond),
    JpCc(Cond),
    CallCc(Cond),
    RetCc(Cond),
}
