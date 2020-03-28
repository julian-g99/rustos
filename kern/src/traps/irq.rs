use alloc::boxed::Box;
use pi::interrupt::Interrupt;

use crate::mutex::Mutex;
use crate::traps::TrapFrame;
use crate::console::kprintln;

pub type IrqHandler = Box<dyn FnMut(&mut TrapFrame) + Send>;
pub type IrqHandlers = [Option<IrqHandler>; Interrupt::MAX];

pub struct Irq(Mutex<Option<IrqHandlers>>);

impl Irq {
    pub const fn uninitialized() -> Irq {
        Irq(Mutex::new(None))
    }

    pub fn initialize(&self) {
        *self.0.lock() = Some([None, None, None, None, None, None, None, None]);
    }

    /// Register an irq handler for an interrupt.
    /// The caller should assure that `initialize()` has been called before calling this function.
    pub fn register(&self, int: Interrupt, handler: IrqHandler) {
        let index = Interrupt::to_index(int);
        //match self.0.lock().iter_mut().next() {
            //None => {},
            //Some(arr) => {
                //(*arr)[index] = Some(handler);
            //}
        //}
        if let Some(ref mut handlers) = *self.0.lock() {
            handlers[i] = Some(handler);
        }
    }

    /// Executes an irq handler for the givven interrupt.
    /// The caller should assure that `initialize()` has been called before calling this function.
    pub fn invoke(&self, int: Interrupt, tf: &mut TrapFrame) {
        let index = Interrupt::to_index(int);
        if let Some(ref mut handlers) = *self.0.lock() {
            if let Some(ref mut handler) = handlers[i] {
                handler(tf);
            }
        }
        //match *self.0.lock() {
            //Some(ref mut handlers) => {
                //match handlers[index] {
                    //Some(ref mut handler) => handler(tf),
                    //None => {}
                //}
            //},
            //None => {}
        //};
    }
}
