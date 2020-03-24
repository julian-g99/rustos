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

fn set_bit(val: u64, bit: u8) -> u64 {
    let mask = 1 << bit;
    val | mask
}

fn clear_bit(val: u64, bit: u8) -> u64 {
    let mask = !(mask << bit);
    val & mask
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

    pub fn set_sp(&mut self, val: u64) {
        self.sp = val;
    }

    pub fn unmask_irq(&mut self) {
        self.spsr = clear_bit(self.spsr, 7);
    }

    pub fn set_aarch64(&mut self) {
        self.spsr = clear_bit(self.spsr, 4);
    }

    pub fn set_el0(&mut self) {
        self.spsr = clear_bit(self.spsr, 2);
        self.spsr = clear_bit(self.spsr, 3);
    }

    pub fn set_lr(&mut self, val: u64) {
        self.lr = val;
    }
}

