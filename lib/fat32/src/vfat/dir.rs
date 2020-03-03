use alloc::string::String;
use alloc::vec::Vec;

use core::fmt;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;
use shim::ioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle, VFat};

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    vfat: HANDLE,
    first_cluster: Cluster,
    metadata: Metadata
}


impl<HANDLE: VFatHandle> Dir<HANDLE> {
    pub fn new(vfat: HANDLE, first_cluster: Cluster, metadata: Metadata) -> Self{
        Dir{vfat, first_cluster, metadata}
    }

    pub fn is_end(&self) -> bool {
        self.metadata.is_end()
    }

    pub fn get_name_utf8(&self) -> io::Result<&str> {
        self.metadata.get_file_string_utf8()
    }
    
    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }
}

pub struct EntryIterator<HANDLE: VFatHandle> {
    //dir: Dir<HANDLE>,
    //curr_entry: Entry<HANDLE>
    chain: Vec<u8>,
    vfat: HANDLE,
    //curr_entry: Entry<HANDLE>
    index: usize
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    file_name: [u8; 8],
    file_extension: [u8; 3],
    attribute: Attributes,
    reserved: u8,
    creation_time_10th_second: u8,
    creation_time: Time,
    creation_date: Date,
    last_access_date: Date,
    first_cluster_high: u16,
    last_modification_time: Time,
    last_modification_date: Date,
    first_cluster_low: u16,
    file_size: u32
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
    other_info: [u8; 20]
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
        println!("The file being queried: {}", name.as_ref().to_str().unwrap());
        use traits::Dir;
        for entry in self.entries()? {
            dbg!(&entry);
            let file_name = String::from(entry.get_name_utf8()?);
            let queried_name = match name.as_ref().to_str() {
                Some(s) => s,
                None => return ioerr!(NotFound, "input in Dir::find() isn't valid unicode")
            };
            if file_name.eq_ignore_ascii_case(queried_name) {
                return Ok(entry);
            }
        }
        ioerr!(NotFound, "failed to find file in Dir::find()")
    }
}

//fn get_byte(num: u32, i: usize) -> u8 {
    //assert!(i <= 3);
    //let mask = 0x011 << ((4 - i) * 8);
    //(num & mask) as u8
//} 

impl<HANDLE: VFatHandle> EntryIterator<HANDLE> {
    fn new_from_dir(root: &Dir<HANDLE>) -> EntryIterator<HANDLE> {
        //unimplemented!("EntryIterator::new_from_dir()")
        //dbg!(root);
        let vfat = root.vfat.clone();
        let mut chain: Vec<u8> = Vec::new();
        vfat.lock(|fat: &mut VFat<HANDLE>| {
            fat.read_chain(root.first_cluster, &mut chain).expect("failed to read chain in EntryIterator::new_from_dir()");
        });

        //let curr_entry = Entry::new(&chain[..32], vfat.clone());

        EntryIterator{vfat, chain, index: 0}
    }
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;
    fn next(&mut self) -> Option<Self::Item> {
        let entry = Entry::new(&(self.chain[self.index .. self.index + 32]), self.vfat.clone());
        if entry.is_end() {
            None
        } else {
            self.index += 32;
            Some(entry)
        }
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
