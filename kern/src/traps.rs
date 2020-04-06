mod frame;
mod syndrome;
mod syscall;

pub mod irq;
pub use self::frame::TrapFrame;

use pi::interrupt::{Controller, Interrupt};

use crate::console::kprintln;
use crate::IRQ;
use crate::shell::shell;

use aarch64::regs::ELR_EL2;

use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {
    //unimplemented!("handle_exception");
    //if info.kind == Kind::Synchronous {
        //let syndrome = Syndrome::from(esr);
        //kprintln!("syndrom: {:?}", syndrome);
        //match syndrome {
            //Syndrome::Brk(val) => {
                //shell("oh no something is wrong: ");
                //let prev_elr = tf.get_elr();
                //tf.set_elr(prev_elr + 4);
            //},
            //_ => {
                //let prev_elr = tf.get_elr();
                //tf.set_elr(prev_elr + 4);
                ////loop {
                    ////aarch64::nop();
                ////}
            //}
        //}
    //} else if info.kind == Kind::Irq {
    if info.kind == Kind::Irq {
        let iter = Interrupt::iter();
        let controller = Controller::new();
        for i in iter {
            if controller.is_pending(*i) {
                IRQ.invoke(*i, tf);
            }
        }
    }
    let prev_elr = tf.get_elr();
    tf.set_elr(prev_elr + 4);
}
