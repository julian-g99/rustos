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
use crate::util::{SliceExt, VecExt};
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status, Metadata};

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
        let mut ebpb_option = None;
        let mut first_sector = 0;
        for i in 0..4 {
            first_sector = mbr.partition_entries[i].relative_sector;
            match BiosParameterBlock::from(&mut device, first_sector as u64) {
                Ok(b) => {
                    ebpb_option = Some(b);
                    break;
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
                        fat_start_sector: first_sector as u64 + ebpb.num_reserved_sectors as u64, data_start_sector: first_sector as u64 + ebpb.num_reserved_sectors as u64 + ebpb.num_fats as u64 * ebpb.sectors_per_fat as u64, rootdir_cluster: Cluster::from(ebpb.rootdir_cluster as u32)};
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
        let mut vector = Vec::new();
        let start_sector = cluster.get_start_sector(self.sectors_per_cluster as u64, self.data_start_sector);

        for i in start_sector..start_sector + self.sectors_per_cluster as u64 {
            vector.extend_from_slice(self.device.get(i)?);
        }

        if vector.len() > buf.len() {
            buf.copy_from_slice(&vector[..buf.len()]);
            return Ok(buf.len());
        } else {
            vector.resize(buf.len(), 0);
            buf.copy_from_slice(vector.as_slice());
            return Ok(vector.len());
        }
    }
    
    /// A method to read all of the clusters chained from a starting cluster
    /// into a vector.
    
    pub fn read_chain(
        &mut self,
        start: Cluster,
        buf: &mut Vec<u8>
    ) -> io::Result<usize> {
        let mut fat_buf = Vec::new();
        for i in self.fat_start_sector..self.fat_start_sector + self.sectors_per_cluster as u64 {
            fat_buf.extend_from_slice(self.device.get(i)?);
        }
        //let fat = unsafe{ fat_buf.cast::<u32>() };

        let mut curr_entry = self.fat_entry(start)?;
        let mut clusters_read = 0;
        loop {
            match curr_entry.status() {
                Status::Eoc(val) => {
                    //TODO: read data from this cluster
                    let start_sector = curr_entry.get_data_sector()?.get_start_sector(self.sectors_per_cluster as u64, self.data_start_sector);

                    for i in start_sector..start_sector + self.sectors_per_cluster as u64 {
                        buf.extend_from_slice(self.device.get(i)?);
                    }
                    clusters_read += 1;
                    println!("EOC: {}", val);
                    return Ok(clusters_read);
                },
                Status::Data(next) => {
                    //TODO: read data from this cluster
                    println!("Data sector: {:?}", next);
                    let start_sector = curr_entry.get_data_sector()?.get_start_sector(self.sectors_per_cluster as u64, self.data_start_sector);

                    for i in start_sector..start_sector + self.sectors_per_cluster as u64 {
                        buf.extend_from_slice(self.device.get(i)?);
                    }

                    curr_entry = self.fat_entry(next)?;
                    clusters_read += 1;
                },
                Status::Free => {
                    //return ioerr!(NotFound, "Encountered a cluster that's neither data nor EOC during read_chain()")
                    return ioerr!(NotFound, "Encountered a free during read_chain()");
                },
                Status::Bad => {
                    //return ioerr!(NotFound, "Encountered a cluster that's neither data nor EOC during read_chain()")
                    return ioerr!(NotFound, "Encountered a bad during read_chain()");
                },
                Status::Reserved => {
                    //return ioerr!(NotFound, "Encountered a cluster that's neither data nor EOC during read_chain()")
                    return ioerr!(NotFound, "Encountered a reserved during read_chain()");
                },
            }
        }
        //let mut fat_buf = Vec::new();
        //for i in self.fat_start_sector..self.sectors_per_fat as u64 {
            //fat_buf.extend_from_slice(self.device.get(i)?);
        //}
        //let fat = unsafe {fat_buf.cast::<u32>()};


    }
    
    ///A method to return a reference to a `FatEntry` for a cluster where the reference points directly into a cached sector.
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> { //TODO: i removed the reference on FatEntry
        let mut buf = Vec::new();
        //let sector = self.device.get(self.fat_start_sector)?;
        for i in self.fat_start_sector..self.fat_start_sector + self.sectors_per_fat as u64 {
            buf.extend_from_slice(self.device.get(i)?);
        }
        //let fat = unsafe {buf.as_slice().cast::<FatEntry>()};
        ////let entry_value = fat[cluster.inner() as usize];
        //Ok(&fat[cluster.inner() as usize])
        unsafe {
            return Ok(&(buf.as_slice().cast::<FatEntry>()[cluster.inner() as usize]));
        }
    }
    
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    //type File = crate::traits::Dummy;
    //type Dir = crate::traits::Dummy;
    //type Entry = crate::traits::Dummy;
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Entry = Entry<HANDLE>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        //unimplemented!("FileSystem::open()")

        let first_cluster = self.lock(|fat: &mut VFat<HANDLE>| -> Cluster {
            fat.rootdir_cluster
        });
        let metadata = self.lock(|fat: &mut VFat<HANDLE>| -> Metadata {
            let mut buf = vec![0u8; 32];
            fat.read_cluster(first_cluster, 0, buf.as_mut_slice()).unwrap();
            Metadata::from(buf.as_slice())
        });

        let dir: Dir<HANDLE> = Dir::new(self.clone(), first_cluster, metadata);
        dir.find(path.as_ref().as_os_str())
    }
}
