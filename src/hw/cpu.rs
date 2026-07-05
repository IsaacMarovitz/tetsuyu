use super::bus::{BusDir, BusMaster, Pins};
use super::interrupt::Interrupts;
use crate::components::mode::GBMode;
use crate::components::registers::{Flags, Registers};
use std::collections::VecDeque;

// --------------------------------------------------------------------------
// Operand selectors
// --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum R8 { B, C, D, E, H, L, A }

#[derive(Clone, Copy)]
pub enum R16 { Bc, De, Hl, Sp }

#[derive(Clone, Copy)]
pub enum R16Stk { Bc, De, Hl, Af }

#[derive(Clone, Copy)]
pub enum Cond { Nz, Z, Nc, C }

#[derive(Clone, Copy)]
pub enum AluOp { Add, Adc, Sub, Sbc, And, Xor, Or, Cp }

#[derive(Clone, Copy)]
pub enum RotOp { Rlc, Rrc, Rl, Rr, Sla, Sra, Swap, Srl }

#[derive(Clone, Copy)]
pub enum Addr { Bc, De, Hl, HighZ, HighC, Wz, Sp }

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

// r-table: index 6 is (HL); handled specially by callers.
fn r8_of(idx: u8) -> R8 {
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

fn rp_of(p: u8) -> R16 {
    match p {
        0 => R16::Bc,
        1 => R16::De,
        2 => R16::Hl,
        _ => R16::Sp,
    }
}

fn rp2_of(p: u8) -> R16Stk {
    match p {
        0 => R16Stk::Bc,
        1 => R16Stk::De,
        2 => R16Stk::Hl,
        _ => R16Stk::Af,
    }
}

fn cc_of(y: u8) -> Cond {
    match y {
        0 => Cond::Nz,
        1 => Cond::Z,
        2 => Cond::Nc,
        _ => Cond::C,
    }
}

fn alu_of(y: u8) -> AluOp {
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

fn rot_of(y: u8) -> RotOp {
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

// --------------------------------------------------------------------------
// Micro-ops and free effects
// --------------------------------------------------------------------------

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
    JrZ,
    SpDec,
    SpInc,
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

pub struct Cpu {
    pub reg: Registers,
    w: u8,
    z: u8,
    ir: u8,
    ime: bool,
    ime_pending: bool,
    halted: bool,
    halt_bug: bool,
    oam_glitch: bool,
    speed_switch: bool,
    micro: VecDeque<MicroOp>,
}

impl Cpu {
    pub fn new(mode: GBMode) -> Self {
        let mut micro = VecDeque::new();
        micro.push_back(MicroOp::Fetch);
        Self {
            reg: Registers::new(mode),
            w: 0,
            z: 0,
            ir: 0,
            ime: false,
            ime_pending: false,
            halted: false,
            halt_bug: false,
            oam_glitch: false,
            speed_switch: false,
            micro,
        }
    }

    // -- register / operand access ----------------------------------------

    fn wz(&self) -> u16 {
        ((self.w as u16) << 8) | self.z as u16
    }

    fn r8(&self, r: R8) -> u8 {
        match r {
            R8::A => self.reg.a,
            R8::B => self.reg.b,
            R8::C => self.reg.c,
            R8::D => self.reg.d,
            R8::E => self.reg.e,
            R8::H => self.reg.h,
            R8::L => self.reg.l,
        }
    }

    fn set_r8(&mut self, r: R8, v: u8) {
        match r {
            R8::A => self.reg.a = v,
            R8::B => self.reg.b = v,
            R8::C => self.reg.c = v,
            R8::D => self.reg.d = v,
            R8::E => self.reg.e = v,
            R8::H => self.reg.h = v,
            R8::L => self.reg.l = v,
        }
    }

    fn r16(&self, r: R16) -> u16 {
        match r {
            R16::Bc => self.reg.get_bc(),
            R16::De => self.reg.get_de(),
            R16::Hl => self.reg.get_hl(),
            R16::Sp => self.reg.sp,
        }
    }

    fn set_r16(&mut self, r: R16, v: u16) {
        match r {
            R16::Bc => self.reg.set_bc(v),
            R16::De => self.reg.set_de(v),
            R16::Hl => self.reg.set_hl(v),
            R16::Sp => self.reg.sp = v,
        }
    }

    fn stk16(&self, r: R16Stk) -> u16 {
        match r {
            R16Stk::Bc => self.reg.get_bc(),
            R16Stk::De => self.reg.get_de(),
            R16Stk::Hl => self.reg.get_hl(),
            R16Stk::Af => self.reg.get_af(),
        }
    }

    fn addr(&self, a: Addr) -> u16 {
        match a {
            Addr::Bc => self.reg.get_bc(),
            Addr::De => self.reg.get_de(),
            Addr::Hl => self.reg.get_hl(),
            Addr::HighZ => 0xFF00 | self.z as u16,
            Addr::HighC => 0xFF00 | self.reg.c as u16,
            Addr::Wz => self.wz(),
            Addr::Sp => self.reg.sp,
        }
    }

    fn byte(&self, b: Byte) -> u8 {
        match b {
            Byte::Reg(r) => self.r8(r),
            Byte::Z => self.z,
            Byte::PcHigh => (self.reg.pc >> 8) as u8,
            Byte::PcLow => self.reg.pc as u8,
            Byte::SpHigh => (self.reg.sp >> 8) as u8,
            Byte::SpLow => self.reg.sp as u8,
            Byte::StkHigh(r) => (self.stk16(r) >> 8) as u8,
            Byte::StkLow(r) => self.stk16(r) as u8,
        }
    }

    fn flag(&self, f: Flags) -> bool {
        self.reg.get_flag(f)
    }

    fn cond(&self, c: Cond) -> bool {
        match c {
            Cond::Nz => !self.flag(Flags::Z),
            Cond::Z => self.flag(Flags::Z),
            Cond::Nc => !self.flag(Flags::C),
            Cond::C => self.flag(Flags::C),
        }
    }

    // -- ALU ---------------------------------------------------------------

    fn alu(&mut self, op: AluOp, v: u8) {
        let a = self.reg.a;
        let carry_in = self.flag(Flags::C) as u8;
        match op {
            AluOp::Add | AluOp::Adc => {
                let cin = if matches!(op, AluOp::Adc) { carry_in } else { 0 };
                let r = a.wrapping_add(v).wrapping_add(cin);
                self.reg.set_flag(Flags::Z, r == 0);
                self.reg.set_flag(Flags::N, false);
                self.reg.set_flag(Flags::H, (a & 0xF) + (v & 0xF) + cin > 0xF);
                self.reg.set_flag(Flags::C, a as u16 + v as u16 + cin as u16 > 0xFF);
                self.reg.a = r;
            }
            AluOp::Sub | AluOp::Sbc | AluOp::Cp => {
                let cin = if matches!(op, AluOp::Sbc) { carry_in } else { 0 };
                let r = a.wrapping_sub(v).wrapping_sub(cin);
                self.reg.set_flag(Flags::Z, r == 0);
                self.reg.set_flag(Flags::N, true);
                self.reg.set_flag(Flags::H, (a & 0xF) < (v & 0xF) + cin);
                self.reg.set_flag(Flags::C, (a as u16) < v as u16 + cin as u16);
                if !matches!(op, AluOp::Cp) {
                    self.reg.a = r;
                }
            }
            AluOp::And => {
                let r = a & v;
                self.reg.set_flag(Flags::Z, r == 0);
                self.reg.set_flag(Flags::N, false);
                self.reg.set_flag(Flags::H, true);
                self.reg.set_flag(Flags::C, false);
                self.reg.a = r;
            }
            AluOp::Xor => {
                let r = a ^ v;
                self.reg.set_flag(Flags::Z, r == 0);
                self.reg.set_flag(Flags::N, false);
                self.reg.set_flag(Flags::H, false);
                self.reg.set_flag(Flags::C, false);
                self.reg.a = r;
            }
            AluOp::Or => {
                let r = a | v;
                self.reg.set_flag(Flags::Z, r == 0);
                self.reg.set_flag(Flags::N, false);
                self.reg.set_flag(Flags::H, false);
                self.reg.set_flag(Flags::C, false);
                self.reg.a = r;
            }
        }
    }

    fn inc8(&mut self, v: u8) -> u8 {
        let r = v.wrapping_add(1);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::H, (v & 0xF) + 1 > 0xF);
        r
    }

    fn dec8(&mut self, v: u8) -> u8 {
        let r = v.wrapping_sub(1);
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.set_flag(Flags::N, true);
        self.reg.set_flag(Flags::H, (v & 0xF) == 0);
        r
    }

    fn add_hl(&mut self, v: u16) {
        let hl = self.reg.get_hl();
        let r = hl.wrapping_add(v);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::H, (hl & 0x0FFF) + (v & 0x0FFF) > 0x0FFF);
        self.reg.set_flag(Flags::C, hl as u32 + v as u32 > 0xFFFF);
        self.reg.set_hl(r);
    }

    fn add_sp_e(&mut self) -> u16 {
        let sp = self.reg.sp;
        let e = self.z as i8 as i16 as u16;
        self.reg.set_flag(Flags::Z, false);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::H, (sp & 0xF) + (e & 0xF) > 0xF);
        self.reg.set_flag(Flags::C, (sp & 0xFF) + (e & 0xFF) > 0xFF);
        sp.wrapping_add(e)
    }

    fn rotate(&mut self, op: RotOp, v: u8) -> u8 {
        let c = self.flag(Flags::C);
        let (r, carry) = match op {
            RotOp::Rlc => (v.rotate_left(1), v & 0x80 != 0),
            RotOp::Rrc => (v.rotate_right(1), v & 0x01 != 0),
            RotOp::Rl => ((v << 1) | c as u8, v & 0x80 != 0),
            RotOp::Rr => ((v >> 1) | ((c as u8) << 7), v & 0x01 != 0),
            RotOp::Sla => (v << 1, v & 0x80 != 0),
            RotOp::Sra => ((v >> 1) | (v & 0x80), v & 0x01 != 0),
            RotOp::Swap => (v.rotate_right(4), false),
            RotOp::Srl => (v >> 1, v & 0x01 != 0),
        };
        self.reg.set_flag(Flags::Z, r == 0);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::C, carry);
        r
    }

    fn bit(&mut self, n: u8, v: u8) {
        self.reg.set_flag(Flags::Z, v & (1 << n) == 0);
        self.reg.set_flag(Flags::N, false);
        self.reg.set_flag(Flags::H, true);
    }

    fn daa(&mut self) {
        let mut a = self.reg.a;
        let mut adjust = 0u8;
        let mut carry = false;
        if self.flag(Flags::H) || (!self.flag(Flags::N) && (a & 0xF) > 9) {
            adjust |= 0x06;
        }
        if self.flag(Flags::C) || (!self.flag(Flags::N) && a > 0x99) {
            adjust |= 0x60;
            carry = true;
        }
        a = if self.flag(Flags::N) {
            a.wrapping_sub(adjust)
        } else {
            a.wrapping_add(adjust)
        };
        self.reg.set_flag(Flags::Z, a == 0);
        self.reg.set_flag(Flags::H, false);
        self.reg.set_flag(Flags::C, carry);
        self.reg.a = a;
    }

    // -- effect execution --------------------------------------------------

    fn exec(&mut self, e: Effect) {
        match e {
            Effect::LdR(d, s) => { let v = self.r8(s); self.set_r8(d, v); }
            Effect::LdRZ(d) => { let v = self.z; self.set_r8(d, v); }
            Effect::Alu(op, s) => { let v = self.r8(s); self.alu(op, v); }
            Effect::AluZ(op) => { let v = self.z; self.alu(op, v); }
            Effect::IncR(r) => { let v = self.inc8(self.r8(r)); self.set_r8(r, v); }
            Effect::DecR(r) => { let v = self.dec8(self.r8(r)); self.set_r8(r, v); }
            Effect::IncZ => self.z = self.inc8(self.z),
            Effect::DecZ => self.z = self.dec8(self.z),
            Effect::Inc16(r) => { let v = self.r16(r); self.note_oam_glitch(v); self.set_r16(r, v.wrapping_add(1)); }
            Effect::Dec16(r) => { let v = self.r16(r); self.note_oam_glitch(v); self.set_r16(r, v.wrapping_sub(1)); }
            Effect::AddHl(r) => { let v = self.r16(r); self.add_hl(v); }
            Effect::AccRot(op) => {
                let v = self.rotate(op, self.reg.a);
                self.reg.a = v;
                self.reg.set_flag(Flags::Z, false);
            }
            Effect::Rot(op, r) => { let v = self.rotate(op, self.r8(r)); self.set_r8(r, v); }
            Effect::RotZ(op) => self.z = self.rotate(op, self.z),
            Effect::Bit(n, r) => { let v = self.r8(r); self.bit(n, v); }
            Effect::BitZ(n) => { let v = self.z; self.bit(n, v); }
            Effect::Res(n, r) => self.set_r8(r, self.r8(r) & !(1 << n)),
            Effect::ResZ(n) => self.z &= !(1 << n),
            Effect::Set(n, r) => self.set_r8(r, self.r8(r) | (1 << n)),
            Effect::SetZ(n) => self.z |= 1 << n,
            Effect::Daa => self.daa(),
            Effect::Cpl => {
                self.reg.a = !self.reg.a;
                self.reg.set_flag(Flags::N, true);
                self.reg.set_flag(Flags::H, true);
            }
            Effect::Scf => {
                self.reg.set_flag(Flags::N, false);
                self.reg.set_flag(Flags::H, false);
                self.reg.set_flag(Flags::C, true);
            }
            Effect::Ccf => {
                let c = self.flag(Flags::C);
                self.reg.set_flag(Flags::N, false);
                self.reg.set_flag(Flags::H, false);
                self.reg.set_flag(Flags::C, !c);
            }
            Effect::LdRpWz(r) => self.set_r16(r, self.wz()),
            Effect::SetStkWz(r) => {
                let v = self.wz();
                match r {
                    R16Stk::Bc => self.reg.set_bc(v),
                    R16Stk::De => self.reg.set_de(v),
                    R16Stk::Hl => self.reg.set_hl(v),
                    R16Stk::Af => self.reg.set_af(v),
                }
            }
            Effect::SetPcWz => self.reg.pc = self.wz(),
            Effect::SetPcHl => self.reg.pc = self.reg.get_hl(),
            Effect::SetPc(v) => self.reg.pc = v,
            Effect::JrZ => {
                let e = self.z as i8 as i16;
                self.reg.pc = (self.reg.pc as i16).wrapping_add(e) as u16;
            }
            Effect::SpDec => { self.note_oam_glitch(self.reg.sp); self.reg.sp = self.reg.sp.wrapping_sub(1); }
            Effect::SpInc => { self.note_oam_glitch(self.reg.sp); self.reg.sp = self.reg.sp.wrapping_add(1); }
            Effect::WzInc => {
                let v = self.wz().wrapping_add(1);
                self.w = (v >> 8) as u8;
                self.z = v as u8;
            }
            Effect::LdSpHl => self.reg.sp = self.reg.get_hl(),
            Effect::AddSpE => { let v = self.add_sp_e(); self.reg.sp = v; }
            Effect::LdHlSpE => { let v = self.add_sp_e(); self.reg.set_hl(v); }
            Effect::Ei => self.ime_pending = true,
            Effect::Di => { self.ime = false; self.ime_pending = false; }
            Effect::Reti => { self.reg.pc = self.wz(); self.ime = true; }
            Effect::Halt => self.halted = true,
            Effect::Stop => self.speed_switch = true,
            Effect::DecodeCb => self.decode_cb(),
            Effect::BranchRel(c) => {
                if self.cond(c) {
                    self.push_front(MicroOp::Exec(Effect::JrZ));
                    self.push_front(MicroOp::Internal);
                }
            }
            Effect::JpCc(c) => {
                if self.cond(c) {
                    self.push_front(MicroOp::Exec(Effect::SetPcWz));
                    self.push_front(MicroOp::Internal);
                }
            }
            Effect::CallCc(c) => {
                if self.cond(c) {
                    self.enqueue_call_front();
                }
            }
            Effect::RetCc(c) => {
                if self.cond(c) {
                    self.enqueue_ret_front();
                }
            }
        }
    }

    // -- queue helpers -----------------------------------------------------

    fn push(&mut self, op: MicroOp) {
        self.micro.push_back(op);
    }

    fn push_front(&mut self, op: MicroOp) {
        self.micro.push_front(op);
    }

    // Front of queue currently holds only the terminal Fetch. Insert the
    // taken-path cycles ahead of it (pushed in reverse).
    fn enqueue_call_front(&mut self) {
        self.push_front(MicroOp::Exec(Effect::SetPcWz));
        self.push_front(MicroOp::Store(Addr::Sp, Byte::PcLow));
        self.push_front(MicroOp::Exec(Effect::SpDec));
        self.push_front(MicroOp::Store(Addr::Sp, Byte::PcHigh));
        self.push_front(MicroOp::Exec(Effect::SpDec));
        self.push_front(MicroOp::Internal);
    }

    fn enqueue_ret_front(&mut self) {
        self.push_front(MicroOp::Exec(Effect::SetPcWz));
        self.push_front(MicroOp::Internal);
        self.push_front(MicroOp::Exec(Effect::SpInc));
        self.push_front(MicroOp::LoadW(Addr::Sp));
        self.push_front(MicroOp::Exec(Effect::SpInc));
        self.push_front(MicroOp::LoadZ(Addr::Sp));
    }

    // -- decode ------------------------------------------------------------

    fn decode(&mut self) {
        let op = self.ir;
        if op == 0xCB {
            self.push(MicroOp::ImmZ);
            self.push(MicroOp::Exec(Effect::DecodeCb));
            return;
        }
        let x = op >> 6;
        let y = (op >> 3) & 7;
        let z = op & 7;
        let p = y >> 1;
        let q = y & 1;

        match x {
            0 => self.decode_x0(y, z, p, q),
            1 => {
                if y == 6 && z == 6 {
                    self.push(MicroOp::Exec(Effect::Halt));
                } else if z == 6 {
                    // LD r,(HL)
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::LdRZ(r8_of(y))));
                } else if y == 6 {
                    // LD (HL),r
                    self.push(MicroOp::Store(Addr::Hl, Byte::Reg(r8_of(z))));
                } else {
                    self.push(MicroOp::Exec(Effect::LdR(r8_of(y), r8_of(z))));
                }
            }
            2 => {
                if z == 6 {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::AluZ(alu_of(y))));
                } else {
                    self.push(MicroOp::Exec(Effect::Alu(alu_of(y), r8_of(z))));
                }
            }
            _ => self.decode_x3(y, z, p, q),
        }
        self.push(MicroOp::Fetch);
    }

    fn decode_x0(&mut self, y: u8, z: u8, p: u8, q: u8) {
        match z {
            0 => match y {
                0 => {} // NOP
                1 => {
                    // LD (a16),SP
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Store(Addr::Wz, Byte::SpLow));
                    self.push(MicroOp::Exec(Effect::WzInc));
                    self.push(MicroOp::Store(Addr::Wz, Byte::SpHigh));
                }
                2 => {
                    // STOP (consumes a padding byte)
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Exec(Effect::Stop));
                }
                3 => {
                    // JR e
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::JrZ));
                }
                _ => {
                    // JR cc,e
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Exec(Effect::BranchRel(cc_of(y - 4))));
                }
            },
            1 => {
                if q == 0 {
                    // LD rp,nn
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Exec(Effect::LdRpWz(rp_of(p))));
                } else {
                    // ADD HL,rp
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::AddHl(rp_of(p))));
                }
            }
            2 => {
                if q == 0 {
                    match p {
                        0 => self.push(MicroOp::Store(Addr::Bc, Byte::Reg(R8::A))),
                        1 => self.push(MicroOp::Store(Addr::De, Byte::Reg(R8::A))),
                        2 => {
                            self.push(MicroOp::Store(Addr::Hl, Byte::Reg(R8::A)));
                            self.push(MicroOp::Exec(Effect::Inc16(R16::Hl)));
                        }
                        _ => {
                            self.push(MicroOp::Store(Addr::Hl, Byte::Reg(R8::A)));
                            self.push(MicroOp::Exec(Effect::Dec16(R16::Hl)));
                        }
                    }
                } else {
                    match p {
                        0 => {
                            self.push(MicroOp::LoadZ(Addr::Bc));
                            self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                        }
                        1 => {
                            self.push(MicroOp::LoadZ(Addr::De));
                            self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                        }
                        2 => {
                            self.push(MicroOp::LoadZ(Addr::Hl));
                            self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                            self.push(MicroOp::Exec(Effect::Inc16(R16::Hl)));
                        }
                        _ => {
                            self.push(MicroOp::LoadZ(Addr::Hl));
                            self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                            self.push(MicroOp::Exec(Effect::Dec16(R16::Hl)));
                        }
                    }
                }
            }
            3 => {
                self.push(MicroOp::Internal);
                if q == 0 {
                    self.push(MicroOp::Exec(Effect::Inc16(rp_of(p))));
                } else {
                    self.push(MicroOp::Exec(Effect::Dec16(rp_of(p))));
                }
            }
            4 => {
                if y == 6 {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::IncZ));
                    self.push(MicroOp::Store(Addr::Hl, Byte::Z));
                } else {
                    self.push(MicroOp::Exec(Effect::IncR(r8_of(y))));
                }
            }
            5 => {
                if y == 6 {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::DecZ));
                    self.push(MicroOp::Store(Addr::Hl, Byte::Z));
                } else {
                    self.push(MicroOp::Exec(Effect::DecR(r8_of(y))));
                }
            }
            6 => {
                if y == 6 {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Store(Addr::Hl, Byte::Z));
                } else {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Exec(Effect::LdRZ(r8_of(y))));
                }
            }
            _ => {
                let eff = match y {
                    0 => Effect::AccRot(RotOp::Rlc),
                    1 => Effect::AccRot(RotOp::Rrc),
                    2 => Effect::AccRot(RotOp::Rl),
                    3 => Effect::AccRot(RotOp::Rr),
                    4 => Effect::Daa,
                    5 => Effect::Cpl,
                    6 => Effect::Scf,
                    _ => Effect::Ccf,
                };
                self.push(MicroOp::Exec(eff));
            }
        }
    }

    fn decode_x3(&mut self, y: u8, z: u8, p: u8, q: u8) {
        match z {
            0 => match y {
                0..=3 => {
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::RetCc(cc_of(y))));
                }
                4 => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Store(Addr::HighZ, Byte::Reg(R8::A)));
                }
                5 => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::AddSpE));
                }
                6 => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::LoadZ(Addr::HighZ));
                    self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                }
                _ => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::LdHlSpE));
                }
            },
            1 => {
                if q == 0 {
                    // POP rp2
                    self.push(MicroOp::LoadZ(Addr::Sp));
                    self.push(MicroOp::Exec(Effect::SpInc));
                    self.push(MicroOp::LoadW(Addr::Sp));
                    self.push(MicroOp::Exec(Effect::SpInc));
                    self.push(MicroOp::Exec(Effect::SetStkWz(rp2_of(p))));
                } else {
                    match p {
                        0 => {
                            // RET
                            self.push(MicroOp::LoadZ(Addr::Sp));
                            self.push(MicroOp::Exec(Effect::SpInc));
                            self.push(MicroOp::LoadW(Addr::Sp));
                            self.push(MicroOp::Exec(Effect::SpInc));
                            self.push(MicroOp::Internal);
                            self.push(MicroOp::Exec(Effect::SetPcWz));
                        }
                        1 => {
                            // RETI
                            self.push(MicroOp::LoadZ(Addr::Sp));
                            self.push(MicroOp::Exec(Effect::SpInc));
                            self.push(MicroOp::LoadW(Addr::Sp));
                            self.push(MicroOp::Exec(Effect::SpInc));
                            self.push(MicroOp::Internal);
                            self.push(MicroOp::Exec(Effect::Reti));
                        }
                        2 => self.push(MicroOp::Exec(Effect::SetPcHl)), // JP HL
                        _ => {
                            // LD SP,HL
                            self.push(MicroOp::Internal);
                            self.push(MicroOp::Exec(Effect::LdSpHl));
                        }
                    }
                }
            }
            2 => match y {
                0..=3 => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Exec(Effect::JpCc(cc_of(y))));
                }
                4 => self.push(MicroOp::Store(Addr::HighC, Byte::Reg(R8::A))), // LDH (C),A
                5 => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Store(Addr::Wz, Byte::Reg(R8::A)));
                }
                6 => {
                    self.push(MicroOp::LoadZ(Addr::HighC));
                    self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                }
                _ => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::LoadZ(Addr::Wz));
                    self.push(MicroOp::Exec(Effect::LdRZ(R8::A)));
                }
            },
            3 => match y {
                0 => {
                    // JP nn
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::SetPcWz));
                }
                6 => self.push(MicroOp::Exec(Effect::Di)),
                7 => self.push(MicroOp::Exec(Effect::Ei)),
                _ => panic!("hw::Cpu: illegal opcode {:#04x}", self.ir),
            },
            4 => match y {
                0..=3 => {
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Exec(Effect::CallCc(cc_of(y))));
                }
                _ => panic!("hw::Cpu: illegal opcode {:#04x}", self.ir),
            },
            5 => {
                if q == 0 {
                    // PUSH rp2
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::SpDec));
                    self.push(MicroOp::Store(Addr::Sp, Byte::StkHigh(rp2_of(p))));
                    self.push(MicroOp::Exec(Effect::SpDec));
                    self.push(MicroOp::Store(Addr::Sp, Byte::StkLow(rp2_of(p))));
                } else if p == 0 {
                    // CALL nn
                    self.push(MicroOp::ImmZ);
                    self.push(MicroOp::ImmW);
                    self.push(MicroOp::Internal);
                    self.push(MicroOp::Exec(Effect::SpDec));
                    self.push(MicroOp::Store(Addr::Sp, Byte::PcHigh));
                    self.push(MicroOp::Exec(Effect::SpDec));
                    self.push(MicroOp::Store(Addr::Sp, Byte::PcLow));
                    self.push(MicroOp::Exec(Effect::SetPcWz));
                } else {
                    panic!("hw::Cpu: illegal opcode {:#04x}", self.ir);
                }
            }
            6 => {
                self.push(MicroOp::ImmZ);
                self.push(MicroOp::Exec(Effect::AluZ(alu_of(y))));
            }
            _ => {
                // RST y*8
                let vector = (y as u16) * 8;
                self.push(MicroOp::Internal);
                self.push(MicroOp::Exec(Effect::SpDec));
                self.push(MicroOp::Store(Addr::Sp, Byte::PcHigh));
                self.push(MicroOp::Exec(Effect::SpDec));
                self.push(MicroOp::Store(Addr::Sp, Byte::PcLow));
                self.push(MicroOp::Exec(Effect::SetPc(vector)));
            }
        }
    }

    // CB-prefixed: the opcode byte is already in Z. Enqueue the operation and
    // the terminal fetch.
    fn decode_cb(&mut self) {
        let op = self.z;
        let x = op >> 6;
        let y = (op >> 3) & 7;
        let z = op & 7;
        let on_hl = z == 6;

        match x {
            0 => {
                let rot = rot_of(y);
                if on_hl {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::RotZ(rot)));
                    self.push(MicroOp::Store(Addr::Hl, Byte::Z));
                } else {
                    self.push(MicroOp::Exec(Effect::Rot(rot, r8_of(z))));
                }
            }
            1 => {
                if on_hl {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::BitZ(y)));
                } else {
                    self.push(MicroOp::Exec(Effect::Bit(y, r8_of(z))));
                }
            }
            2 => {
                if on_hl {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::ResZ(y)));
                    self.push(MicroOp::Store(Addr::Hl, Byte::Z));
                } else {
                    self.push(MicroOp::Exec(Effect::Res(y, r8_of(z))));
                }
            }
            _ => {
                if on_hl {
                    self.push(MicroOp::LoadZ(Addr::Hl));
                    self.push(MicroOp::Exec(Effect::SetZ(y)));
                    self.push(MicroOp::Store(Addr::Hl, Byte::Z));
                } else {
                    self.push(MicroOp::Exec(Effect::Set(y, r8_of(z))));
                }
            }
        }
        self.push(MicroOp::Fetch);
    }

    // -- boundary / interrupt ---------------------------------------------

    pub fn at_instruction_boundary(&self) -> bool {
        matches!(self.micro.front(), Some(MicroOp::Fetch))
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn wake(&mut self) {
        self.halted = false;
    }

    /// Arm the HALT bug: the next opcode fetch reads the byte after HALT
    /// without advancing PC, so it is decoded twice.
    pub fn trigger_halt_bug(&mut self) {
        self.halt_bug = true;
    }

    /// Record that a 16-bit inc/dec acted on an address in the OAM region.
    /// The DMG glitches OAM when this happens during mode 2; the motherboard
    /// forwards the event and the PPU decides whether it corrupts.
    fn note_oam_glitch(&mut self, addr: u16) {
        if (0xFE00..=0xFEFF).contains(&addr) {
            self.oam_glitch = true;
        }
    }

    pub fn take_oam_glitch(&mut self) -> bool {
        std::mem::take(&mut self.oam_glitch)
    }

    pub fn take_speed_switch(&mut self) -> bool {
        std::mem::take(&mut self.speed_switch)
    }

    pub fn ime(&self) -> bool {
        self.ime
    }

    pub fn offer_interrupt(&mut self, pending: Interrupts) -> Option<Interrupts> {
        if !self.ime || !self.at_instruction_boundary() {
            return None;
        }
        let (vector, bit) = pending.highest()?;
        self.ime = false;
        self.micro.clear();
        // Dispatch is 5 M-cycles on hardware, measured from *after* the
        // interrupted instruction's opcode fetch. Offering here at `front ==
        // Fetch` discards that fetch M-cycle (the prefetch of the next opcode),
        // so one extra internal cycle stands in for it to keep the total right.
        self.push(MicroOp::Internal);
        self.push(MicroOp::Internal);
        self.push(MicroOp::Internal);
        self.push(MicroOp::Exec(Effect::SpDec));
        self.push(MicroOp::Store(Addr::Sp, Byte::PcHigh));
        self.push(MicroOp::Exec(Effect::SpDec));
        self.push(MicroOp::Store(Addr::Sp, Byte::PcLow));
        self.push(MicroOp::Exec(Effect::SetPc(vector)));
        self.push(MicroOp::Fetch);
        Some(bit)
    }

    // -- M-cycle interface -------------------------------------------------

    pub fn run_free_acts(&mut self) {
        while matches!(self.micro.front(), Some(MicroOp::Exec(_))) {
            if let Some(MicroOp::Exec(e)) = self.micro.pop_front() {
                self.exec(e);
            }
        }
    }

    pub fn setup(&mut self, pins: &mut Pins) {
        pins.master = BusMaster::Cpu;
        match self.micro.front().expect("micro-op queue underflow") {
            MicroOp::Fetch | MicroOp::ImmZ | MicroOp::ImmW => {
                pins.address = self.reg.pc;
                pins.dir = BusDir::Read;
            }
            MicroOp::LoadZ(a) | MicroOp::LoadW(a) => {
                pins.address = self.addr(*a);
                pins.dir = BusDir::Read;
            }
            MicroOp::Store(a, b) => {
                pins.address = self.addr(*a);
                pins.data = self.byte(*b);
                pins.dir = BusDir::Write;
            }
            MicroOp::Internal => pins.dir = BusDir::Idle,
            MicroOp::Exec(_) => unreachable!("Exec drained by run_free_acts"),
        }
    }

    pub fn complete(&mut self, pins: &Pins) -> bool {
        match self.micro.pop_front().expect("micro-op queue underflow") {
            MicroOp::Fetch => {
                self.ir = pins.data;
                if self.halt_bug {
                    self.halt_bug = false;
                } else {
                    self.reg.pc = self.reg.pc.wrapping_add(1);
                }
                if self.ime_pending {
                    self.ime_pending = false;
                    self.ime = true;
                }
                self.decode();
                true
            }
            MicroOp::ImmZ => {
                self.z = pins.data;
                self.reg.pc = self.reg.pc.wrapping_add(1);
                false
            }
            MicroOp::ImmW => {
                self.w = pins.data;
                self.reg.pc = self.reg.pc.wrapping_add(1);
                false
            }
            MicroOp::LoadZ(_) => { self.z = pins.data; false }
            MicroOp::LoadW(_) => { self.w = pins.data; false }
            MicroOp::Store(_, _) => false,
            MicroOp::Internal => false,
            MicroOp::Exec(_) => unreachable!("Exec drained in setup"),
        }
    }
}
