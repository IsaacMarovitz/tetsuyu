struct Registers {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u8,
    sp: u8,
}

struct CPU {
    reg: Registers
}

impl CPU {
}