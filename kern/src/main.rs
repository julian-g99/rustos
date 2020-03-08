#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;
extern crate fat32;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;

use console::kprintln;

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.
//extern crate pi;
use pi::timer::spin_sleep;
use pi::gpio::Gpio;
use pi::gpio::Function;
use core::time::Duration;
use pi::uart::MiniUart;
use core::fmt::Write;

//NOTE: code from skeleton
use allocator::Allocator;
use fs::FileSystem;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {
    // FIXME: Start the shell.
    use fat32::traits::FileSystem as FsTrait;
    use fat32::traits::Dir as DirTrait;
    use fat32::vfat::{Dir, Entry};
    use core::iter::Iterator;
    use core::iter;
    use shim::path::Path;
    let duration = Duration::from_millis(1000);
    spin_sleep(duration);


    //NOTE: this is code from the lab3 skeleton
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }
    loop {
        shell::shell("> "); //this way it never returns
    }
}
