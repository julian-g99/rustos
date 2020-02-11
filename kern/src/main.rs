#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

pub mod console;
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

//const GPIO_BASE: usize = 0x3F000000 + 0x200000;

//const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
//const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
//const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

unsafe fn kmain() -> ! {
    // FIXME: Start the shell.
    let duration = Duration::from_millis(1000);
     ////getting the gpio
    //let mut pin_16 = Gpio::new(16).into_output();
    
    //GPIO_FSEL1.write_volatile(1 << 18);
    //loop {
        ////GPIO_SET0.write_volatile(1 << 16);
        //pin_16.set();
        //spin_sleep(duration);
        ////GPIO_CLR0.write_volatile(1 << 16);
        //pin_16.clear();
        //spin_sleep(duration);
    //}
    
    spin_sleep(duration);
    loop {
        shell::shell("> ");
    }
    //let mut uart = MiniUart::new();
    //loop {
        ////let read_byte = uart.read_byte();
        ////uart.write_byte(read_byte);
    //}
}
