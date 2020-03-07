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
    
    kprintln!("Welcome to cs3210!");
    kprintln!("Files in the root: ");
    
    let dir = match(&FILESYSTEM).open_dir(Path::new("/")) {
        Err(_) => panic!("Failed to read dir entries"),
        Ok(d) => d
    };
    //let iter = dir.entries().expect();
    //let _:() = iter.next();
    kprintln!("fs works");
    
    use crate::fat32::traits::Entry as EntryTrait;
    //kprintln!("num entries: {}", dir.entries().expect("reeee").len());
    let mut iter = dir.entries().expect("entry iterator");
    kprintln!("first entry: {}", iter.next().expect("first entry").name());
    //for e in dir.entries().expect("hello") {
        //panic!("one time for the one time");
        //kprintln!("entry name: {}", e.name());
    //}
    panic!("dir success");

    //shell::shell("> ");
    //above skeleton code
    panic!("pls man");
    loop {

    }
}
