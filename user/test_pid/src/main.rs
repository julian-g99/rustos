#![feature(asm)]
#![no_std]
#![no_main]

mod cr0;

use kernel_api::println;
use kernel_api::syscall::{getpid, time};

fn main() {
    println!("pid is: {}", getpid());
}
