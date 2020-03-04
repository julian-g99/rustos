use crate::vfat::*;
use core::fmt;

use self::Status::*;

use shim::io;
use shim::ioerr;

#[derive(Debug, PartialEq)]
pub enum Status {
    /// The FAT entry corresponds to an unused (free) cluster.
    Free,
    /// The FAT entry/cluster is reserved.
    Reserved,
    /// The FAT entry corresponds to a valid data cluster. The next cluster in
    /// the chain is `Cluster`.
    Data(Cluster),
    /// The FAT entry corresponds to a bad (disk failed) cluster.
    Bad,
    /// The FAT entry corresponds to a valid data cluster. The corresponding
    /// cluster is the last in its chain.
    Eoc(u32),
}

#[repr(C, packed)]
pub struct FatEntry(pub u32);

impl FatEntry {
    /// Returns the `Status` of the FAT entry `self`.
    pub fn status(&self) -> Status {
        let cleared_val = self.0 & 0x0fffffff;
        //let cleared_val = self.0;
        if cleared_val == 0 {
            Free
        } else if cleared_val == 1 || (cleared_val >= 0x0FFFFFF0 && cleared_val <= 0x0FFFFFF6) {
            Reserved
        } else if cleared_val == 0x0FFFFFF7 {
            Bad
        } else if cleared_val >= 0x0FFFFFF8 && cleared_val <= 0x0FFFFFFF {
            Eoc(cleared_val) //TODO: use actual Eoc
        } else {
            Data(Cluster::from(self.0))
        }
    }

    //pub fn get_data_sector(&self) -> io::Result<Cluster> {
        //match self.status() {
            //Data(cluster) => Ok(cluster),
            //_ => ioerr!(NotFound, "invalid fat entry")
        //}
    //}
}

impl fmt::Debug for FatEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FatEntry")
            .field("value", &{ self.0 })
            .field("status", &self.status())
            .finish()
    }
}
