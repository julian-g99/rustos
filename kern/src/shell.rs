use stack_vec::StackVec;
use core::str::from_utf8;
use core::fmt::Write;

use crate::console::{kprint, kprintln, CONSOLE};

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell(prefix: &str) -> ! {
    loop {
        kprint!("{}", prefix);
        let mut buffer = [0u8; 512];
        let mut input = StackVec::new(&mut buffer);
        let mut console = CONSOLE.lock();
        loop {
            let read_byte = console.read_byte();
            //buffer[index] = read_byte;
            //index += 1;
            if read_byte == '\r' as u8 || read_byte == '\n' as u8 {
                kprintln!();
                break;
            } else if read_byte == 8 || read_byte == 127 {
                if input.len() > 0 {
                    input.pop();
                    console.write_byte(8);
                    console.write_byte(b' ');
                    console.write_byte(8);
                }
            } else {
                input.push(read_byte);
                console.write_byte(read_byte);
            }
        }
        let mut stack_backend = [""; 64];
        let command = Command::parse(from_utf8(input.as_slice()).unwrap(), &mut stack_backend);
        match command {
            Ok(c) => {
                if c.path() == "echo" {
                    for (i, v) in c.args.iter().enumerate() {
                        if i != 0 {
                            console.write_str(v);
                            console.write_str(" ");
                        }
                    }
                    kprintln!();
                } else {
                    kprintln!("unknown command: {}", c.path());
                }
            },
            Err(_) => {
                kprintln!("uh oh something went wrong");
            }
        }
    }
}
