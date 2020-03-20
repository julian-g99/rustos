use shim::io;
use shim::path::{Path, PathBuf};
use alloc::string::String;

use stack_vec::StackVec;
use core::str::from_utf8;
use core::fmt::Write;
use shim::io::Read;

use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry, Timestamp, Metadata};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

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

fn pwd(cwd: &PathBuf) {
    kprintln!("{}", cwd.as_os_str().to_str().expect("cwd isn't valid utf-8"));
}

fn hash_entry<T: Entry>(hash: &mut String, entry: &T, show_hidden: bool) -> ::core::fmt::Result {
    use core::fmt::Write;

    fn write_bool(to: &mut String, b: bool, c: char) -> ::core::fmt::Result {
        if b {
            write!(to, "{}", c)
        } else {
            write!(to, "-")
        }
    }

    fn write_timestamp<T: Timestamp>(to: &mut String, ts: T) -> ::core::fmt::Result {
        write!(
            to,
            "{:02}/{:02}/{} {:02}:{:02}:{:02} ",
            ts.month(),
            ts.day(),
            ts.year(),
            ts.hour(),
            ts.minute(),
            ts.second()
        )
    }

    if show_hidden || !entry.metadata().hidden() {
        write_bool(hash, entry.is_dir(), 'd')?;
        write_bool(hash, entry.is_file(), 'f')?;
        write_bool(hash, entry.metadata().read_only(), 'r')?;
        write_bool(hash, entry.metadata().hidden(), 'h')?;
        write!(hash, "\t")?;

        write_timestamp(hash, entry.metadata().created())?;
        write_timestamp(hash, entry.metadata().modified())?;
        write_timestamp(hash, entry.metadata().accessed())?;
        write!(hash, "\t")?;

        write!(hash, "{}\n", entry.name())?;
        Ok(())
    } else {
        Ok(())
    }
}


fn ls(cwd: &PathBuf, query: StackVec<&str>) {
    if query.len() > 3 {
        kprintln!("Query can have at most three arguments.");
        return;
    }
    let mut output = String::new();
    let mut final_dir = cwd.clone();
    let mut show_hidden = false;
    for i in 1..query.len() {
        let arg = query[i];
        if arg == "-a" {
            show_hidden = true;
        } else {
            use shim::path::Component;
            let query_path = PathBuf::from(String::from(arg));
            'outer: for comp in query_path.components() {
                match comp {
                    Component::RootDir => {
                        final_dir = PathBuf::from("/");
                    },
                    Component::CurDir => {
                        continue;
                    },
                    Component::ParentDir => {
                        final_dir.pop();
                    },
                    Component::Normal(name) => {
                        let dir = FILESYSTEM.open_dir(&final_dir).expect("Failed to read queried directory");
                        let iterator = dir.entries().expect("Falied to create iterator");
                        for entry in iterator {
                            if entry.name() == name && entry.is_file() {
                                kprintln!("ls query path contains a file");
                                return;
                            }
                            if entry.name() == name {
                                final_dir.push(name);
                                continue 'outer;
                            }
                        }
                        kprintln!("{:?} is not found", name);
                        return;
                    },
                    _ => {
                        kprintln!("Invalid ls target");
                        return;
                    }
                }
            }
        }
    }
    let dir = FILESYSTEM.open_dir(&final_dir).expect("Failed to read contents of ls final dir");
    let iterator = dir.entries().expect("Failed to create iterator");
    for entry in iterator {
        hash_entry(&mut output, &entry, show_hidden);
    }
    kprintln!("{}", output);
}


fn cat(cwd: &PathBuf, query: StackVec<&str>) -> () {
    let dir = FILESYSTEM.open_dir(cwd).expect("Failed to read content of cwd");

    let mut buf = String::new();
    for (i, q) in query.iter().enumerate() {
        let iterator = dir.entries().expect("Failed to create iterator from cwd");
        if i == 0 {
            continue;
        }
        for entry in iterator {
            let mut entry_path = cwd.clone();
            entry_path.push(entry.name());
            //kprintln!("entry name: {}, is_dir: {}", entry.name(), entry.is_dir());
            if entry.name() == *q {
                if entry.is_dir() {
                    kprintln!("{} is directory", q);
                    return;
                }
                entry.into_file().unwrap().read_to_string(&mut buf);
            }
        }
    }
    kprintln!("{}", buf);

}

fn cd(cwd: &mut PathBuf, query: StackVec<&str>) {
    if query.len() == 1 {
        *cwd = PathBuf::from("/");
        return;
    }
    let dest = query[1];
    let path = PathBuf::from(String::from(dest));
    use shim::path::Component;
    for comp in path.components() {
        match comp {
            Component::RootDir => {
                *cwd = PathBuf::from("/");
            },
            Component::CurDir => {
                continue;
            },
            Component::ParentDir => {
                cwd.pop();
            },
            Component::Normal(name) => {
                let dir = FILESYSTEM.open_dir(&cwd).expect("Failed to read content of cwd");
                let iterator = dir.entries().expect("Failed to create iterator from cwd");
                for entry in iterator {
                    if entry.is_dir() && entry.name() == name {
                        cwd.push(name);
                        return;
                    }
                }
                kprintln!("Directory not found. You changed directory to the last viable dir in the given path");
            },
            _ => {
                kprintln!("Invalid cd target");
            }
        }
    }
}

fn clear_screen() {
    for i in 0..500 {
        kprintln!();
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell(prefix: &str) {
    //clear_screen();
    kprintln!("Hello! Welcome to the shell!");
    let mut cwd = PathBuf::from("/");
    'outer: loop {
        kprintln!("{}", cwd.display());
        kprint!("{}", prefix);
        let mut buffer = [0u8; 512];
        let mut input = StackVec::new(&mut buffer);
        let mut console = CONSOLE.lock();
        'inner: loop {
            let read_byte = console.read_byte();
            if read_byte == '\r' as u8 || read_byte == '\n' as u8 {
                kprintln!();
                break 'inner;
            } else if read_byte == 8 || read_byte == 127 {
                if input.len() > 0 {
                    input.pop();
                    console.write_byte(8);
                    console.write_byte(b' ');
                    console.write_byte(8);
                }
            } else {
                if input.push(read_byte).is_err() {
                    kprintln!();
                    kprintln!("Input exceeding 512 characters. Stop!");
                    continue 'outer;
                }
                console.write_byte(read_byte);
            }
        }
        let mut stack_backend = [""; 64];
        let command_string = from_utf8(input.as_slice());
        match command_string {
            Err(_) => {
                kprintln!("Please give commands in valid utf-8 characters");
            },
            Ok(c) => {
                let command = Command::parse(from_utf8(input.as_slice()).unwrap(), &mut stack_backend);
                match command {
                    Ok(c) => {
                        match c.path() {
                            "echo" => {
                                for (i, v) in c.args.iter().enumerate() {
                                    if i != 0 {
                                        console.write_str(v);
                                        console.write_str(" ");
                                    }
                                }
                                kprintln!();
                            },
                            "exit" => {
                                return;
                            },
                            "pwd" => {
                                pwd(&cwd);
                            },
                            "ls" => {
                                ls(&cwd, c.args);
                            },
                            "cat" => {
                                if c.args.len() < 2 {
                                    kprintln!("No argument given to cat");
                                    continue;
                                }
                                cat(&cwd, c.args);
                            },
                            "cd" => {
                                cd(&mut cwd, c.args);
                            },
                            "clear" => {
                                clear_screen();
                            }
                            _ => {
                                kprintln!("unknown command: {}", c.path());
                            }
                        }
                    },
                    Err(Error::TooManyArgs) => {
                        kprintln!("Too many arguments. Stop!");
                    },
                    Err(Error::Empty) => {
                        kprintln!("Empty command");
                    },
                    Err(_) => {
                        kprintln!("Unknown parsing error");
                    }
                }
            }
        }
    }
}
