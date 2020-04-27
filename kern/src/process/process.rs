use alloc::boxed::Box;
use alloc::fmt;
use shim::io;
use shim::path::Path;

use aarch64;

use crate::param::*;
use crate::process::{Stack, State};
use crate::traps::TrapFrame;
use crate::vm::*;
use crate::console::kprintln;
use crate::FILESYSTEM;
use fat32::traits::{FileSystem, Entry, File};
use io::Read;
use kernel_api::{OsError, OsResult};

const STACK_SIZE: usize = 1 << 20; //CHECK: is this still true for this phase?

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
//#[derive(Debug)]
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<UserPageTable>,
    /// The scheduling state of the process.
    pub state: State,
    curr_img: VirtualAddr
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new() -> OsResult<Process> {
        let frame: TrapFrame = Default::default();
        let context = Box::new(frame);
        let stack = match Stack::new() {
            Some(stack) => stack,
            None => return Err(OsError::NoMemory)
        };
        let state = State::Ready;
        let vmap = Box::new(UserPageTable::new());
        let curr_img = Process::get_image_base();
        Ok(Process{ context, stack, state, vmap , curr_img})
        //Ok(Process{ context, stack, state })
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::do_load(pn)?;

        //FIXME: Set trapframe for the process.
        p.context.set_sp(Self::get_stack_top().as_u64());
        p.context.set_elr(Self::get_image_base().as_u64());
        p.context.set_ttbr0(VMM.get_baddr().as_u64());
        p.context.set_ttbr1(p.vmap.get_baddr().as_u64());
        p.context.set_aarch64();
        p.context.set_el0();
        p.context.unmask_irq();
        p.context.set_fiq(); //F bit
        p.context.set_serror_interrupt(); //A bit
        p.context.set_d(); //D bit

        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        // creating the process
        let mut process = Self::new()?;

        // allocate stack in virtual space
        let stack_base = Self::get_stack_base();
        if !stack_base.is_aligned(PAGE_SIZE) {
            panic!("stack base is not aligned to page size");
        }
        let stack_page = process.vmap.alloc(stack_base, PagePerm::RW);

        // open file and read content into virtual space
        let mut file_entry = FILESYSTEM.open_file(pn)?;
        let mut read_so_far = 0;

        while read_so_far < file_entry.size() {
            let page = process.vmap.alloc(process.curr_img, PagePerm::RW);
            read_so_far += file_entry.read(page)? as u64;
            process.curr_img += VirtualAddr::from(PAGE_SIZE);
        }

        Ok(process)

        //match fs_entry.into_file() {
            //None => {
                //return Err(OsError::IoError);
            //},
            //Some(mut f) => {
                //let mut buf = [0u8; PAGE_SIZE];
                //let mut total = 0;
                //let mut bytes_read = f.read(&mut buf).expect("failed to read"); // FIXME: parse this properly
                //while bytes_read != 0 {
                    //let vaddr = VirtualAddr::from(USER_IMG_BASE+total);
                    ////if !vaddr.is_aligned(PAGE_SIZE) {
                        ////panic!("vaddr is not aligned to page size");
                    ////} else {
                        ////panic!("is aligned");
                    ////}
                    //let mut page = process.vmap.alloc(vaddr, PagePerm::RWX);
                    //page[..bytes_read].copy_from_slice(&buf[..bytes_read]);
                    //bytes_read = f.read(&mut buf).expect("failed to read");

                    //total += bytes_read;
                //}
            //}
        //}

        //Ok(process)
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE + USER_MAX_VM_SIZE - 1) //otherwise it would be 0
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
        VirtualAddr::from(USER_STACK_BASE - USER_STACK_BASE % PAGE_SIZE)
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack.
    pub fn get_stack_top() -> VirtualAddr {
        VirtualAddr::from(USER_STACK_BASE - 16 + PAGE_SIZE)
    }

    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
        //kprintln!("checking ready");
        use core::mem::replace;
        use State::*;
        use crate::console::kprintln;
        match self.state {
            Ready => {
                //kprintln!("it's ready doe");
                return true;
            },
            Waiting(_) => {
                let mut old_state = replace(&mut self.state, State::Ready);
                let mut wait_done: bool = false;
                if let Waiting(ref mut f) = old_state {
                    wait_done = f(self);
                }

                if !wait_done {
                    self.state = old_state;
                }


                return wait_done;
            },
            _ => false
        }
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "context: {:?}", self.context)
    }
}
