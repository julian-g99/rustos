use alloc::boxed::Box;
use core::time::Duration;
use pi::timer::current_time;

use crate::console::{CONSOLE, kprint, kprintln};
use crate::process::{State, Process};
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;

const SLEEP: u16 = NR_SLEEP as u16;
const WRITE: u16 = NR_WRITE as u16;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    use crate::console::kprintln;
    //kprintln!("ms is: {}", ms);

    let end_time = current_time() + Duration::from_millis(ms as u64);
    let poll_fn = Box::new(move |proc: &mut Process| -> bool {
        let curr_time = current_time();
        if curr_time < end_time {
            return false;
        } else {
            proc.context.set_x_register(7, OsError::Ok as u64);
            proc.context.set_x_register(0, (curr_time - end_time).as_millis() as u64);
            return true;
        }

    });
    SCHEDULER.switch(State::Waiting(poll_fn), tf);
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {
    let curr_time = current_time();
    let full_secs = curr_time.as_secs();
    let nanos = (curr_time - Duration::from_secs(full_secs)).as_nanos();
    tf.set_x_register(7, OsError::Ok as u64);
    tf.set_x_register(0, full_secs as u64);
    tf.set_x_register(1, nanos as u64);
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    SCHEDULER.kill(tf);
    tf.set_x_register(7, OsError::Ok as u64);
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    let c = b as char;
    if c.is_ascii() {
        kprint!("{}", c);
        tf.set_x_register(7, OsError::Ok as u64);
    } else {
        tf.set_x_register(7, OsError::InvalidArgument as u64);
    }
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    let pid = tf.get_tpidr();
    tf.set_x_register(0, pid);
    tf.set_x_register(7, OsError::Ok as u64);
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    use crate::console::kprintln;
    //unimplemented!("handle_syscall()")
    match num {
        SLEEP => {
            let millis = tf.get_x_register(0) as u32;
            sys_sleep(millis, tf);
        },
        WRITE => {
            let input = tf.get_x_register(0) as u8;
            sys_write(input, tf);
        },
        _ => {}
    }
}
