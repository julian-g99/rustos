use core::fmt;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    head: u8,
    sector_cylinder: u16
}

// FIXME: implement Debug for CHS
impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "head: {:?}, sector and cylinder: {:?}", self.head, self.sector_cylinder)
    }
}

//const_assert_size!(CHS, 3);

#[repr(C, packed)]
pub struct PartitionEntry {
    boot_indicator: u8,
    head_chs: CHS,
    partition_type: u8,
    end_chs: CHS,
    relative_sector: u32,
    total_sectors: u32
}

// FIXME: implement Debug for PartitionEntry
impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boot indcator: {:?}, head chs: {:?}, partition type: {:?}, \
            end chs: {:?}, relative sector: {:?}, total sectors: {:?}"
            ,self.boot_indicator, &{self.head_chs}, self.partition_type,
            &{self.end_chs}, &{self.relative_sector}, &{self.total_sectors})
    }
}


//const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    partition_entries: [PartitionEntry; 4],
    valid_siganture: u16
}

// FIXME: implemente Debug for MaterBootRecord
impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unique disk id: {:?}, partition entries: {:?}, \
            valid sector signature bytes: {:?}",
             self.disk_id, self.partition_entries, &{self.valid_siganture})
    }
}

//const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        unimplemented!("MasterBootRecord::from()")
    }
}
