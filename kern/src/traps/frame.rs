use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    elr: u64,
    spsr: u64,
    sp: u64,
    tpidr: u64,

    q_registers: [i128; 32],

    x_registers: [i64; 31],

    xzr: u64
}

impl TrapFrame {
    pub fn new(elr: u64, spsr: u64, sp: u64, tpidr: u64,
        q_registers: [i128; 32], x_registers: [i64; 31],
        xzr: u64) -> Self {
        TrapFrame{ elr, spsr, sp, tpidr, q_registers, x_registers, xzr }
    }
    pub fn get_elr(&self) -> u64 {
        self.elr
    }

    pub fn set_elr(&mut self, val: u64) {
        self.elr = val;
    }
}

