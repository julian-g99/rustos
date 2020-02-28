use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle, VFat};

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    first_cluster: Cluster,
    metadata: Metadata
}

pub struct EntryIterator<HANDLE: VFatHandle> {
    dir: Dir<HANDLE>,
    curr_entry: Entry<HANDLE>
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    file_name: [u8; 8],
    file_extension: [u8; 3],
    attribute: Attributes,
    reserved: u8,
    creation_time_10th_second: u16,
    creation_time: Time,
    creation_date: Date,
    last_access_date: Date,
    first_cluster_high: u16,
    last_modification_time: Time,
    last_modification_date: Date,
    first_cluster_low: u16,
    file_size: u16
}

//const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    file_name: [u8; 10],
    attribute: Attributes,
    file_type: u8,
    checksum: u8,
    second_name: [u8; 12],
    reserved: u16,
    third_name: [u8; 4]
}

//const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    file_name_or_extension: [u8; 11],
    attribute: Attributes,
    other_info: [u8; 24]
}

//const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        unimplemented!("Dir::find()")
    }
}

//fn get_byte(num: u32, i: usize) -> u8 {
    //assert!(i <= 3);
    //let mask = 0x011 << ((4 - i) * 8);
    //(num & mask) as u8
//} 

impl<HANDLE: VFatHandle> EntryIterator<HANDLE> {
    fn new_from_dir(root: &Dir<HANDLE>) -> EntryIterator<HANDLE> {
        unimplemented!("EntryIterator::new_from_dir()")
        //let reached_end = false;
        //let dir_start = root.first_cluster;

        //loop {
            //let mut cluster_chain: Vec<u8> = Vec::new();
            ////root.vfat.read_chain(root.dir.first_cluster, &mut cluster_chain);
            //root.vfat.lock(|fat: &mut VFat<HANDLE>| {
                //fat.read_chain(root.first_cluster, &mut cluster_chain);
            //});
            //let entries = VecExt::cast::<VFatDirEntry>(cluster_chain);
            //for entry in entries {
                //let unknown = unsafe {entry.unknown};
                //match attribute {
                    //0x10 => {

                    //}
                //}
            //}
        //}
    }
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!("DirIterator::next()")
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    // FIXME: Implement `trait::Dir` for `Dir`.
    type Entry = Entry<HANDLE>;
    type Iter = EntryIterator<HANDLE>;

    fn entries(&self) -> io::Result<Self::Iter> {
        //TODO: implement
        Ok(EntryIterator::new_from_dir(self))
    }
}
