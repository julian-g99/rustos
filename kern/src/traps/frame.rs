use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    TTBR1: u64,
    TTBR0: u64,
    elr: u64,
    spsr: u64,
    sp: u64,
    tpidr: u64,

    q_registers: [u128; 32],

    x_registers: [u64; 31],
}

fn set_bit(val: u64, bit: u8) -> u64 {
    let mask = 1 << bit;
    val | mask
}

fn clear_bit(val: u64, bit: u8) -> u64 {
    let mask = !(1 << bit);
    val & mask
}

impl TrapFrame {
    //pub fn new(elr: u64, spsr: u64, sp: u64, tpidr: u64,
        //q_registers: [u128; 32], x_registers: [u64; 31],
        //xzr: u64) -> Self {
        //TrapFrame{ elr, spsr, sp, tpidr, q_registers, x_registers, xzr }
    //}
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
        self.x_registers[30] = val;
    }

    pub fn set_tpidr(&mut self, val: u64) {
        self.tpidr = val;
    }

    pub fn get_tpidr(&mut self) -> u64 {
        self.tpidr
    }

    pub fn set_x_register(&mut self, index: usize, val: u64) {
        self.x_registers[index] = val;
    }

    pub fn get_x_register(&self, index: usize) -> u64 {
        self.x_registers[index]
    }

    pub fn set_ttbr0(&mut self, val: u64) {
        self.TTBR0 = val;
    }

    pub fn set_ttbr1(&mut self, val: u64) {
        self.TTBR1 = val;
    }

    pub fn set_fiq(&mut self) {
        self.spsr = set_bit(self.spsr, 6);
    }

    pub fn set_serror_interrupt(&mut self) {
        self.spsr = set_bit(self.spsr, 8);
    }

    pub fn set_d(&mut self) {
        self.spsr = set_bit(self.spsr, 9);
    }
}
