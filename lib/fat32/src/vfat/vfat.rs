use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr = match MasterBootRecord::from(&mut device) {
            Ok(m) => m,
            Err(e) => {
                return Err(Error::Mbr(e));
            }
        };

        //TODO: change this to not use first partition found
        let ebpb_option = None;
        let first_sector = 0;
        for i in 0..4 {
            first_sector = mbr.partition_entries[i].relative_sector;
            match BiosParameterBlock::from(&mut device, first_sector as u64) {
                Ok(b) => {
                    ebpb_option = Some(b);
                },
                Err(e) => {
                    return Err(e);
                }
            }
        };
        let ebpb = ebpb_option.expect("ebpb unwrap failed");
        //let first_sector = mbr.partition_entries[0].relative_sector; //CHECK: is "start of disk" always 0/
        //let ebpb = match BiosParameterBlock::from(&mut device, first_sector as u64) {
            //Ok(b) => b,
            //Err(e) => {
                //return Err(e);
            //}
        //};
        let partition = Partition{start: first_sector as u64 + ebpb.num_reserved_sectors as u64, num_sectors: ebpb.sectors_per_fat as u64, sector_size: ebpb.bytes_per_sector as u64};
        let vfat = VFat{phantom: PhantomData, device: CachedPartition::new(device, partition), bytes_per_sector: ebpb.bytes_per_sector,
                        sectors_per_cluster: ebpb.sectors_per_cluster, sectors_per_fat: ebpb.sectors_per_fat,
                        fat_start_sector: first_sector as u64 + ebpb.num_reserved_sectors as u64, data_start_sector: first_sector as u64 + ebpb.num_reserved_sectors as u64 + ebpb.num_fats as u64, rootdir_cluster: Cluster(ebpb.rootdir_cluster as u32)};
        Ok(HANDLE::new(vfat))
    }

    // TODO: The following methods may be useful here:
    //
    ///A method to read from an offset of a cluster into a buffer.
    pub fn read_cluster( //CHECK: should this be public?
        &mut self,
        cluster: Cluster,
        offset: usize, //CHECK: what is the unit of this offset
        buf: &mut [u8]
    ) -> io::Result<usize> {
        //CHECK: is cluster always bigger than sector
        let sector = cluster.0 * self.sectors_per_cluster as u32 + offset as u32;
        match self.device.get(sector as u64) {
            Err(e) => return Err(e),
            Ok(s) => {
                buf.copy_from_slice(s);
                return Ok(buf.len());
            }
        }
    }
    
    /// A method to read all of the clusters chained from a starting cluster
    /// into a vector.
    
    pub fn read_chain(
        &mut self,
        start: Cluster,
        buf: &mut Vec<u8>
    ) -> io::Result<usize> {
        unimplemented!("VFat::read_chain()")
    }
    
    ///A method to return a reference to a `FatEntry` for a cluster where the reference points directly into a cached sector.
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<FatEntry> { //TODO: i removed the reference on FatEntry
        let sector = self.device.get(self.fat_start_sector)?;
        let entry_value = sector[cluster.0 as usize * size_of::<u32>()];
        Ok(FatEntry(entry_value as u32))
    }
    
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = crate::traits::Dummy;
    type Dir = crate::traits::Dummy;
    type Entry = crate::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }
}
