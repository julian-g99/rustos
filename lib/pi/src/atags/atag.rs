use crate::atags::raw;

pub use crate::atags::raw::{Core, Mem};

/// An ATAG.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Atag {
    Core(raw::Core),
    Mem(raw::Mem),
    Cmd(&'static str),
    Unknown(u32),
    None,
}

impl Atag {
    /// Returns `Some` if this is a `Core` ATAG. Otherwise returns `None`.
    pub fn core(self) -> Option<Core> {
        match self {
            Atag::Core(c) => Some(c),
            _ => None
        }
    }

    /// Returns `Some` if this is a `Mem` ATAG. Otherwise returns `None`.
    pub fn mem(self) -> Option<Mem> {
        match self {
            Atag::Mem(m) => Some(m),
            _ => None
        }
    }

    /// Returns `Some` with the command line string if this is a `Cmd` ATAG.
    /// Otherwise returns `None`.
    pub fn cmd(self) -> Option<&'static str> {
        match self {
            Atag::Cmd(cmd) => Some(cmd),
            _ => None
        }
    }
}

// FIXME: Implement `From<&raw::Atag> for `Atag`.
impl From<&'static raw::Atag> for Atag {
    fn from(atag: &'static raw::Atag) -> Atag {
        // FIXME: Complete the implementation below.
        unsafe {
            let data_ptr = (&atag.dwords as *const u32).add(2);
            match (atag.tag, &atag.kind) {
                (raw::Atag::CORE, &raw::Kind { core }) => {
                    //if atag.dwords == 2 {
                        ////TODO: what if there's no data attached?
                    //} else {
                        return Atag::Core(*(data_ptr as *const raw::Core));
                    //}
                },
                (raw::Atag::MEM, &raw::Kind { mem }) => {
                    return Atag::Mem(*(data_ptr as *const raw::Mem));
                },
                (raw::Atag::CMDLINE, &raw::Kind { ref cmd }) => {
                    let mut size = 0;
                    let first_byte = data_ptr as *const u8;
                    while *(first_byte.add(size)) != '\0' as u8 {
                        size += 1;
                    }
                    let s = core::str::from_utf8(core::slice::from_raw_parts(first_byte, size));
                    return Atag::Cmd(s.unwrap());
                },
                (raw::Atag::NONE, _) => {
                    return Atag::None;
                },
                (id, _) => {
                    return Atag::Unknown(id);
                }
            }
        }
    }
}
