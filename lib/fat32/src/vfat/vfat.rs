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
        //dbg!(ebpb);
        let partition = Partition{start: first_sector as u64, num_sectors: ebpb.total_logical_sectors(), sector_size: ebpb.bytes_per_sector as u64};
        let vfat = VFat{phantom: PhantomData, device: CachedPartition::new(device, partition), bytes_per_sector: ebpb.bytes_per_sector,
                        sectors_per_cluster: ebpb.sectors_per_cluster, sectors_per_fat: ebpb.sectors_per_fat,
                        fat_start_sector: ebpb.num_reserved_sectors as u64, data_start_sector: ebpb.num_reserved_sectors as u64 + ebpb.num_fats as u64 * ebpb.sectors_per_fat as u64, rootdir_cluster: Cluster::from(ebpb.rootdir_cluster as u32)};
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
        if offset >= self.sectors_per_cluster as usize {
            return ioerr!(InvalidInput, "offset given to read_cluster() is too big");
        }
        let mut vec = Vec::new();

        for i in offset as u64..self.sectors_per_cluster as u64 {
            let sector = cluster.get_start_sector(self.sectors_per_cluster as u64, self.data_start_sector) + i;
            let data = self.device.get(sector)?;
            vec.extend_from_slice(data);
        }

        if vec.len() >= buf.len() {
            buf.copy_from_slice(&vec[..buf.len()]);
            return Ok(buf.len());
        } else {
            buf[..vec.len()].copy_from_slice(vec.as_slice());
            return Ok(vec.len());
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
        let mut bytes_read: usize = 0;
        loop {
            buf.resize(buf.len() + self.sectors_per_cluster as usize * self.bytes_per_sector as usize, 0);
            bytes_read += self.read_cluster(curr_cluster, 0, &mut buf[bytes_read..])?;
            match self.fat_entry(curr_cluster)?.status() {
                Status::Eoc(_) => {
                    return Ok(bytes_read);
                },
                Status::Data(next) => {
                    curr_cluster = next;
                },
                Status::Free => {
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
    }
    
    ///A method to return a reference to a `FatEntry` for a cluster where the reference points directly into a cached sector.
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> { //TODO: i removed the reference on FatEntry
        let entries_per_sector = self.bytes_per_sector as u32 / 4;
        let sector_index = self.fat_start_sector + (cluster.inner() / entries_per_sector) as u64;
        //if (sector_index - self.fat_start_sector) > self.sectors_per_fat as u64 {
            //println!("oh no");
        //}
        let offset = cluster.inner() % entries_per_sector;
        let buf = self.device.get(sector_index)?;
        let fat: &[FatEntry] = unsafe{ buf.cast() };
        Ok(&fat[offset as usize])
    }

    pub fn bytes_per_cluster(&self) -> u64 {
        self.bytes_per_sector as u64 * self.sectors_per_cluster as u64
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
        if path.as_ref().is_relative() {
            return ioerr!(Other, "Path given to open is relative");
        }

        let first_cluster = self.lock(|fat: &mut VFat<HANDLE>| -> Cluster {
            fat.rootdir_cluster
        });
        let metadata = self.lock(|fat: &mut VFat<HANDLE>| -> Metadata {
            let mut buf = vec![0u8; 32];
            fat.read_cluster(first_cluster, 0, buf.as_mut_slice()).unwrap();
            Metadata::from(buf.as_slice())
        });

        let mut iter = path.as_ref().components();
        let mut curr_dir: Dir<HANDLE> = Dir::new(self.clone(), first_cluster, metadata.clone());
        let root: Self::Dir = Dir::new(self.clone(), first_cluster, metadata.clone());
        let mut stack = Vec::new();
        stack.push(Entry::new_from_dir(root));
        iter.next();
        for comp in iter {
            match comp {
                Component::Normal(s) => {
                    match curr_dir.find(s)? {
                        Entry::File(f) => {
                            stack.push(Entry::File(f));
                        },
                        Entry::Dir(d) => {
                            stack.push(Entry::Dir(d.clone()));
                            curr_dir = d;
                        }
                    }
                },
                _ => {
                    panic!("Not normal");
                }
            }
        }
        return Ok(stack.pop().expect("stack is empty"));
    }
}
