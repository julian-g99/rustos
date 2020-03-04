use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Component;
use shim::path::Path;

use crate::mbr::{MasterBootRecord, PartitionEntry};
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
            if !mbr.partition_entries[i].is_fat() {
                continue;
            }
            match BiosParameterBlock::from(&mut device, first_sector as u64) {
                Ok(b) => {
                    if b.good_signature() {
                        ebpb_option = Some(b);
                        break;
                    }
                },
                Err(e) => {
                    continue;
                }
            }
        };
        let ebpb = ebpb_option.expect("ebpb unwrap failed");
        println!("device sector size: {}", device.sector_size());
        dbg!(ebpb);
        //let partition = Partition{start: first_sector as u64 + ebpb.num_reserved_sectors as u64, num_sectors: ebpb.sectors_per_fat as u64, sector_size: ebpb.bytes_per_sector as u64};
        let partition = Partition{start: first_sector as u64, num_sectors: ebpb.sectors_per_fat as u64, sector_size: ebpb.bytes_per_sector as u64};
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
        let start_sector = cluster.get_start_sector(self.sectors_per_cluster as u64, self.data_start_sector);

        //for i in start_sector..start_sector + self.sectors_per_cluster as u64 {
            //vector.extend_from_slice(self.device.get(i)?);
        //}
        let sector = self.device.get(start_sector + offset as u64)?;

        if sector.len() >= buf.len() {
            buf.copy_from_slice(&sector[..buf.len()]);
            return Ok(buf.len());
        } else {
            //sector.resize(buf.len(), 0);
            let sub_buffer = &mut buf[..sector.len()];
            sub_buffer.copy_from_slice(sector);
            return Ok(sector.len());
        }
    }
    
    /// A method to read all of the clusters chained from a starting cluster
    /// into a vector.
    
    pub fn read_chain(
        &mut self,
        start: Cluster,
        buf: &mut Vec<u8>
    ) -> io::Result<usize> {
        let mut curr_cluster = start;
        let mut clusters_read = 0;
        loop {
            match self.fat_entry(curr_cluster)?.status() {
                Status::Eoc(v) => {
                    buf.resize(buf.len() + self.sectors_per_cluster as usize * self.bytes_per_sector as usize, 0);
                    let num = self.read_cluster(curr_cluster, 0, buf.as_mut_slice())?;
                    clusters_read += 1;
                    return Ok(clusters_read);
                },
                Status::Data(next) => {
                    //TODO: read data from this cluster
                    //let start_sector = curr_cluster.get_start_sector(self.sectors_per_cluster as u64, self.data_start_sector);
                    buf.resize(buf.len() + self.sectors_per_cluster as usize * self.bytes_per_sector as usize, 0);
                    self.read_cluster(curr_cluster, 0, buf.as_mut_slice())?;
                    curr_cluster = next;
                    clusters_read += 1;
                },
                Status::Free => {
                    println!("free cluster: {}", curr_cluster.inner());
                    return ioerr!(NotFound, "Encountered a free during read_chain()");
                },
                Status::Bad => {
                    return ioerr!(NotFound, "Encountered a bad during read_chain()");
                },
                Status::Reserved => {
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
        println!("current cluster: {}", cluster.inner());
        let entries_per_sector = self.bytes_per_sector as u32 / 4;
        let sector_index = self.fat_start_sector + (cluster.inner() / entries_per_sector) as u64;
        let offset = cluster.inner() % entries_per_sector;
        let buf = self.device.get(sector_index)?;
        let fat: &[FatEntry] = unsafe{ buf.cast() };
        Ok(&fat[offset as usize])
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
        //TODO: atm only deals with absolute path

        let first_cluster = self.lock(|fat: &mut VFat<HANDLE>| -> Cluster {
            fat.rootdir_cluster
        });
        let metadata = self.lock(|fat: &mut VFat<HANDLE>| -> Metadata {
            let mut buf = vec![0u8; 32];
            fat.read_cluster(first_cluster, 0, buf.as_mut_slice()).unwrap();
            Metadata::from(buf.as_slice())
        });

        let mut curr_dir: Dir<HANDLE> = Dir::new(self.clone(), first_cluster, metadata);
        for comp in path.as_ref().components() {
            if comp == Component::RootDir {
                return Ok(Entry::new_from_dir(curr_dir));
            }
        }
        curr_dir.find(path.as_ref().as_os_str())
    }
}
