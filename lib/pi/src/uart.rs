use core::fmt;
use core::time::Duration;

use shim::io;
use shim::const_assert_size;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile, Reserved};

use crate::timer;
use crate::common::IO_BASE;
use crate::gpio::{Gpio, Function};

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // FIXME: Declare the "MU" registers from page 8.
    // FIXME: change the types to be minimal
    IO: Volatile<u8>,
    _r0: [Reserved<u8>; 3],
    IER: Volatile<u8>,
    _r1: [Reserved<u8>; 3],
    IIR: Volatile<u8>,
    _r2: [Reserved<u8>; 3],
    LCR: Volatile<u8>,
    _r3: [Reserved<u8>; 3],
    MCR: Volatile<u8>,
    _r4: [Reserved<u8>; 3],
    LSR: ReadVolatile<u8>,
    _r5: [Reserved<u8>; 3],
    MSR: ReadVolatile<u8>,
    _r6: [Reserved<u8>; 3],
    SCRATCH: Volatile<u8>,
    _r7: [Reserved<u8>; 3],
    CNTL: Volatile<u8>,
    _r8: [Reserved<u8>; 3],
    STAT: ReadVolatile<u32>,
    BAUD: Volatile<u16>,
    _r9: [Reserved<u8>; 2],
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<Duration>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new() -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*AUX_ENABLES).or_mask(1);
            &mut *(MU_REG_BASE as *mut Registers)
        };

        // FIXME: Implement remaining mini UART initialization.
        //registers.LCR.write(0b011);
        registers.LCR.or_mask(0b011);
        registers.BAUD.write(270);
        let gpio15 = Gpio::new(15);
        gpio15.into_alt(Function::Alt5);
        let gpio14 = Gpio::new(14);
        gpio14.into_alt(Function::Alt5);
        //registers.CNTL.write(0b011);
        registers.CNTL.or_mask(0b011);

        MiniUart{registers: registers, timeout: None}
    }

    /// Set the read timeout to `t` duration.
    pub fn set_read_timeout(&mut self, t: Duration) {
        self.timeout = Some(t);
    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        loop {
            //testing if the 5th bit of LSR is set
            if self.registers.LSR.has_mask(LsrStatus::TxAvailable as u8) {
                //TODO: write
                self.registers.IO.write(byte);
                break;
            }
        }
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        //self.registers.LSR.read() & 0b01 != 0
        self.registers.LSR.has_mask(LsrStatus::DataReady as u8)
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        match self.timeout {
            Some(t) => {
                let time_stop = timer::current_time() + t;
                loop {
                    if timer::current_time() < time_stop {
                        if self.has_byte() {
                            return Ok(());
                        }
                    } else {
                        return Err(());
                    }
                }
            },
            None => {
                loop {
                    if self.has_byte() {
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        loop {
            if self.has_byte() {
                return self.registers.IO.read();
            }
        }
    }
}

// FIXME: Implement `fmt::Write` for `MiniUart`. A b'\r' byte should be written
// before writing any b'\n' byte.
impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.as_bytes() {
            if *b == '\n' as u8 {
                self.write_byte('\r' as u8);
                self.write_byte(*b);
            } else {
                self.write_byte(*b);
            }
            //match b {
                //b if *b == '\n' as u8 => {
                    //self.write_byte('\r' as u8);
                    //self.write_byte(*b);
                //},
                //_ => {
                    //self.write_byte(*b);
                //}
            //}
        }
        Ok(())
    }
}

mod uart_io {
    use super::io;
    use super::MiniUart;
    use volatile::prelude::*;

    // FIXME: Implement `io::Read` and `io::Write` for `MiniUart`.
    //
    // The `io::Read::read()` implementation must respect the read timeout by
    // waiting at most that time for the _first byte_. It should not wait for
    // any additional bytes but _should_ read as many bytes as possible. If the
    // read times out, an error of kind `TimedOut` should be returned.
    //
    // The `io::Write::write()` method must write all of the requested bytes
    // before returning.
    impl io::Read for MiniUart {
        //TODO: implement this. also how do i know if there are more than one byte to read?
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
            let mut size: usize = 0;
            match self.wait_for_byte() {
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "Read out of time")),
                Ok(_) =>{
                    loop {
                        if self.has_byte() {
                            buf[size] = self.read_byte();
                            size += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            return Ok(size);
        }
    }

    impl io::Write for MiniUart {
        fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
            let mut size: usize = 0;
            for i in 0..buf.len() {
                self.write_byte(buf[i]);
                size += 1;
            }
            Ok(size)
        }

        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(()) //CHECK: this should be ok because we block until everything is written?
        }
    }
}
