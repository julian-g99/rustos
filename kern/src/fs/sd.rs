use core::time::Duration;
use shim::io;
use shim::ioerr;

use fat32::traits::BlockDevice;
use crate::spin_sleep;

extern "C" {
    /// A global representing the last SD controller error that occured.
    static sd_err: i64;

    /// Initializes the SD card controller.
    ///
    /// Returns 0 if initialization is successful. If initialization fails,
    /// returns -1 if a timeout occured, or -2 if an error sending commands to
    /// the SD controller occured.
    fn sd_init() -> i32;

    /// Reads sector `n` (512 bytes) from the SD card and writes it to `buffer`.
    /// It is undefined behavior if `buffer` does not point to at least 512
    /// bytes of memory. Also, the caller of this function should make sure that
    /// `buffer` is at least 4-byte aligned.
    ///
    /// On success, returns the number of bytes read: a positive number.
    ///
    /// On error, returns 0. The true error code is stored in the `sd_err`
    /// global. `sd_err` will be set to -1 if a timeout occured or -2 if an
    /// error sending commands to the SD controller occured. Other error codes
    /// are also possible but defined only as being less than zero.
    fn sd_readsector(n: i32, buffer: *mut u8) -> i32;
}

// FIXME: Define a `#[no_mangle]` `wait_micros` function for use by `libsd`.
// The `wait_micros` C signature is: `void wait_micros(unsigned int);`
#[no_mangle]
fn wait_micros(micros: u32) {
    let duration = Duration::from_micros(micros as u64 * 200);
    spin_sleep(duration);
}

/// A handle to an SD card controller.
#[derive(Debug)]
pub struct Sd;

impl Sd {
    /// Initializes the SD card controller and returns a handle to it.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization. We can enforce the requirement in safe Rust code
    /// with atomic memory access, but we can't use it yet since we haven't
    /// written the memory management unit (MMU).
    pub unsafe fn new() -> Result<Sd, io::Error> {
        //unimplemented!("Sd::new()")
        match sd_init() {
            0 => Ok(Sd),
            -1 => ioerr!(TimedOut, "sd init timed out"),
            -2 => ioerr!(Other, "failed to send command to SD controller"),
            _ => ioerr!(Other, "Sd::new() got output that's neither 0, -1, nor -2")
        }
    }
}

impl BlockDevice for Sd {
    /// Reads sector `n` from the SD card into `buf`. On success, the number of
    /// bytes read is returned.
    ///
    /// # Errors
    ///
    /// An I/O error of kind `InvalidInput` is returned if `buf.len() < 512` or
    /// `n > 2^31 - 1` (the maximum value for an `i32`).
    ///
    /// An error of kind `TimedOut` is returned if a timeout occurs while
    /// reading from the SD card.
    ///
    /// An error of kind `Other` is returned for all other errors.
    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
        //unimplemented!("Sd::read_sector()")
        

        //checking align
        let ptr = buf.as_mut_ptr();
        unsafe {
            if ptr.align_offset(4) != 0 {
                panic!("Buffer given to read_sector() isn't aligned to 4 bytes");
            }
            if buf.len() < 512 {
                panic!("Buffer given to read_sector() is less than 512 bytes");
            }
            match sd_readsector(n as i32, ptr) {
                n if n > 0 => {
                    return Ok(n as usize);
                },
                0 => {
                    if sd_err == -1 {
                        return ioerr!(TimedOut, "sd_readsector() timed out");
                    } else if sd_err == -2 {
                        return ioerr!(Other, "failed to send command to SD controller");
                    } else {
                        return ioerr!(Other, "sd_readsector() error but other error code");
                    }
                },
                _ => return ioerr!(Other, "sd_readsector() returned negative value")
            }
        }
    }

    fn write_sector(&mut self, _n: u64, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("SD card and file system are read only")
    }
}
