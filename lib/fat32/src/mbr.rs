use core::fmt;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
pub struct CHS {
    head: u8,
    sector_cylinder: u16
}

// FIXME: implement Debug for CHS
impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "head: {:?}, sector and cylinder: {:?}", self.head, &{self.sector_cylinder})
    }
}

//const_assert_size!(CHS, 3);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct PartitionEntry {
    boot_indicator: u8,
    head_chs: CHS,
    partition_type: u8,
    end_chs: CHS,
    pub relative_sector: u32,
    total_sectors: u32
}

impl PartitionEntry {
    pub fn is_fat(&self) -> bool {
        self.partition_type == 0xB || self.partition_type == 0xC
    }
}

// FIXME: implement Debug for PartitionEntry
impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boot indcator: {:?},\nhead chs: {:?},\n partition type: {:?},\n \
            end chs: {:?},\n relative sector: {:?},\n total sectors: {:?}"
            ,self.boot_indicator, &{self.head_chs}, self.partition_type,
            &{self.end_chs}, &{self.relative_sector}, &{self.total_sectors})
    }
}


//const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    pub partition_entries: [PartitionEntry; 4], //TODO: should I do this?
    valid_signature: u16
}

// FIXME: implemente Debug for MaterBootRecord
impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unique disk id: {:?},\npartition entries: {:?},\n \
            valid sector signature bytes: {:?}",
            self.disk_id, self.partition_entries, &{self.valid_signature})
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
        //let sector_size = device.sector_size();
        //let buf: &mut[u8] = &mut ([0u8; 512]); //CHECK: is it ok to just use 512 here?
        let mut arr = [0u8; 512];
        let buf = &mut arr[..];
        match device.read_sector(0, buf) {
            Err(e) => {
                return Err(Error::Io(e));
            },
            Ok(_) => {
                let mbr = unsafe{ *(buf.as_mut_ptr() as *mut MasterBootRecord) };
                //let mbr = unsafe {core::mem::transmute::<}
                if mbr.valid_signature != 0xAA55 {
                    return Err(Error::BadSignature);
                }
                for i in 0..4 {
                    let partition = mbr.partition_entries[i];
                    if partition.boot_indicator != 0 && partition.boot_indicator != 0x80 {
                        return Err(Error::UnknownBootIndicator(i as u8));
                    }
                }
                return Ok(mbr);
            }
        }
    }
}
