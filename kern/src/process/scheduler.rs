use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::fmt;

use aarch64::*;
use core::time::Duration;
use pi::timer::tick_in;
use crate::IRQ;
use pi::interrupt::{Interrupt, Controller};

use crate::console::{kprintln, kprint};

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
        let ret = self.switch_to(tf);
        ret
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
            //asm!("mov sp, $0
                  //bl context_restore

                  //mov x0, sp
                  //and x0, x0, $2
                  //add x0, x0, $1
                  //mov sp, x0

                  //mov lr, xzr
                  //mov x0, xzr
                  //eret"
                  //:
                  //:"r"(tf), "r"(PAGE_SIZE), "r"(PAGE_MASK)
                  //:"x0"
                  //:"volatile");
        }

        loop {}
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        let mut scheduler = Scheduler::new();
        *self.0.lock() = Some(scheduler);

        use shim::path::PathBuf;
        let proc1 = Process::load(PathBuf::from("/fib.bin")).unwrap();
        self.add(proc1);

        let proc2 = Process::load(PathBuf::from("/fib.bin")).unwrap();
        self.add(proc2);

        let proc3 = Process::load(PathBuf::from("/fib.bin")).unwrap();
        self.add(proc3);

        let proc4 = Process::load(PathBuf::from("/fib.bin")).unwrap();
        self.add(proc4);

        let proc5 = Process::load(PathBuf::from("/fib.bin")).unwrap();
        self.add(proc5);
    }


    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    pub fn test_phase_3(&self, proc: &mut Process){
        use crate::vm::{VirtualAddr, PagePerm};

        let mut page = proc.vmap.alloc(
         VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);

        let text = unsafe {
         core::slice::from_raw_parts(test_user_process as *const u8, 24)
        };

        page[0..24].copy_from_slice(text);

    }
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
        let id = match self.last_id {
            None => {0},
            Some(v) => v.checked_add(1)?
        };

        process.context.set_tpidr(id);
        self.processes.push_back(process);
        self.last_id = Some(id);
        self.last_id
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        let mut del_idx = self.processes.len();
        for i in 0..self.processes.len() {
            let proc = self.processes.get(i).expect("can't get process");
            if proc.context.get_tpidr() == tf.get_tpidr() {
                del_idx = i;
                break;
            }
        }

        if del_idx == self.processes.len() {
            return false;
        }

        let mut proc = self.processes.remove(del_idx).expect("failed to remove");
        proc.state = new_state;
        *proc.context = *tf;
        self.processes.push_back(proc);
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
        // print scheduling queue


        let mut del_idx = self.processes.len();
        for i in 0..self.processes.len() {
            let proc = self.processes.get_mut(i).expect("can't get proc"); //is ready needs mut
            if proc.is_ready() {
                del_idx = i;
                break;
            }
        }

        if del_idx == self.processes.len() {
            return None;
        }

        let mut proc = self.processes.remove(del_idx).expect("failed to remove");
        proc.state = State::Running;
        let id = proc.context.get_tpidr();
        *tf = *proc.context;
        self.processes.push_front(proc);
        return Some(id);
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
    let res = sleep(Duration::from_secs(5));
    kprintln!("milliseconds passed: {}", res.unwrap().as_millis());
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
        let ms = 100000;
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
