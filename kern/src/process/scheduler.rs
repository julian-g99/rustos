use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::fmt;

use aarch64::*;
use core::time::Duration;
use pi::timer::tick_in;
use crate::IRQ;
use pi::interrupt::{Interrupt, Controller};

use crate::console::kprintln;

use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::traps::TrapFrame;
use crate::VMM;

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }
            aarch64::wfe();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        use crate::SCHEDULER;
        let mut controller = Controller::new();
        controller.enable(Interrupt::Timer1);
        tick_in(TICK);
        IRQ.register(Interrupt::Timer1, Box::new(|frame| {
            tick_in(TICK);
            SCHEDULER.switch(State::Ready, frame);
        }));

        let mut process1 = Process::new().expect("not enough memory to start process in GlobalScheduler::new()");
        process1.context.set_sp(process1.stack.top().as_u64());
        process1.context.set_elr(start_shell1 as u64);
        process1.context.set_aarch64();
        process1.context.set_el0();
        process1.context.unmask_irq();

        let mut process2 = Process::new().expect("not enough memory to start process in GlobalScheduler::new()");
        process2.context.set_sp(process2.stack.top().as_u64());
        process2.context.set_elr(start_shell2 as u64);
        process2.context.set_aarch64();
        process2.context.set_el0();
        process2.context.unmask_irq();

        let mut process3 = Process::new().expect("not enough memory to start process in GlobalScheduler::new()");
        process3.context.set_sp(process3.stack.top().as_u64());
        process3.context.set_elr(start_shell3 as u64);
        process3.context.set_aarch64();
        process3.context.set_el0();
        process3.context.unmask_irq();

        let mut process4 = Process::new().expect("not enough memory to start process in GlobalScheduler::new()");
        process4.context.set_sp(process4.stack.top().as_u64());
        process4.context.set_elr(start_shell4 as u64);
        process4.context.set_aarch64();
        process4.context.set_el0();
        process4.context.unmask_irq();

        self.add(process1);
        self.add(process2);
        self.add(process3);
        self.add(process4);

        //let process_id = SCHEDULER.switch_to(&mut trap_frame);
        let mut tf = Box::new(TrapFrame::default());
        self.switch_to(tf.as_mut());
        unsafe {
            asm!("mov sp, $0
                  bl context_restore

                  adr x0, _start
                  mov sp, x0

                  mov lr, xzr
                  eret"
                  :
                  :"r"(tf)
                  :"x0"
                  :"volatile");
        }

        loop {}
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        let mut scheduler = Scheduler::new();
        *self.0.lock() = Some(scheduler);
    }


    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    // pub fn test_phase_3(&self, proc: &mut Process){
    //     use crate::vm::{VirtualAddr, PagePerm};
    //
    //     let mut page = proc.vmap.alloc(
    //         VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);
    //
    //     let text = unsafe {
    //         core::slice::from_raw_parts(test_user_process as *const u8, 24)
    //     };
    //
    //     page[0..24].copy_from_slice(text);
    // }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        let processes = VecDeque::new();
        let last_id = None;
        
        Self{ processes, last_id }
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        if self.last_id.is_none() {
            self.last_id = Some(0);
            self.processes.push_back(process);
            return self.last_id;
        }
        let id = match self.last_id.unwrap().checked_add(1) {
            None => return None,
            Some(v) => v
        };

        process.context.set_tpidr(id);
        self.processes.push_back(process);
        kprintln!("length: {}", self.processes.len());
        Some(id)
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        let mut curr_proc = match self.processes.pop_front() {
            None => return false,
            Some(p) => p
        };
        curr_proc.state = new_state;
        curr_proc.context = Box::new(*tf);

        self.processes.push_back(curr_proc);
        return true;
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        use State::*;
        for i in 0..self.processes.len() {
            let mut curr = match self.processes.remove(i) {
                None => return None,
                Some(p) => p
            };

            if curr.is_ready() {
                let id = curr.context.get_tpidr();
                *tf = *curr.context;
                curr.state = State::Running;
                self.processes.push_front(curr);
                return Some(id);
            } else {
                self.processes.insert(i, curr);
            }
        }

        return None;
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        match self.schedule_out(State::Dead, tf) {
            false => None,
            true => {
                let mut curr = self.processes.pop_back().unwrap();
                let id = curr.context.get_tpidr();
                drop(curr);
                return Some(id);
            }
        }
    }
}

pub extern "C" fn start_shell1() {
    use crate::shell;
    use kernel_api::syscall::sleep;
    let res = sleep(Duration::from_millis(50));
    //kprintln!("milliseconds passed: {}", res.unwrap().as_millis());
    shell::shell("user1> ");
}


pub extern "C" fn start_shell2() {
    use crate::shell;
    shell::shell("user2> ");
}


pub extern "C" fn start_shell3() {
    use crate::shell;
    shell::shell("user3> ");
}


pub extern "C" fn start_shell4() {
    use crate::shell;
    shell::shell("user4> ");
}

pub extern "C" fn  test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}

