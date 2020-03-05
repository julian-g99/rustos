use alloc::string::String;
use alloc::vec::Vec;

use core::fmt;

use shim::const_assert_size;
use std::char::decode_utf16;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;
use shim::ioerr;

use crate::traits;
use crate::util::{SliceExt, VecExt};
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle, VFat};

#[derive(Debug, Clone)]
pub struct Dir<HANDLE: VFatHandle> {
    vfat: HANDLE,
    first_cluster: Cluster,
    metadata: Metadata,
    name: String
}


impl<HANDLE: VFatHandle> Dir<HANDLE> {
    pub fn new(vfat: HANDLE, first_cluster: Cluster, metadata: Metadata) -> Self{
        let name = metadata.get_short_name().to_string();
        Dir{vfat, first_cluster, metadata, name}
    }

    pub fn is_end(&self) -> bool {
        self.metadata.is_end()
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
    //pub fn get_name_utf8(&self) -> io::Result<&str> {
        ////self.metadata.get_file_string_utf8(
        //Ok(self.name.as_str())
    //}
    
    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn from_regular_entry(handle: HANDLE, entry: VFatRegularDirEntry, name: String) -> Self {
        let vfat = handle.clone();
        //let first_cluster = Cluster::from(((entry.first_cluster_high as u32) << 16) + (entry.first_cluster_low as u32));
        let first_cluster = entry.get_cluster();
        let metadata = entry.get_metadata();
        Dir{vfat, first_cluster, metadata, name}
    }
}

pub struct EntryIterator<HANDLE: VFatHandle> {
    //dir: Dir<HANDLE>,
    //curr_entry: Entry<HANDLE>
    chain: Vec<VFatDirEntry>,
    vfat: HANDLE,
    //curr_entry: Entry<HANDLE>
    index: usize,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
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

impl VFatRegularDirEntry {
    pub fn get_metadata(&self) -> Metadata {
        let creation_time = Timestamp::new(self.creation_date, self.creation_time);
        let last_access_date = Timestamp::new(self.creation_date, Default::default());
        let last_modification_date = Timestamp::new(self.last_modification_date, self.last_modification_time);
        Metadata::new(&self.file_name, &self.file_extension, self.attribute, creation_time, last_access_date, last_modification_date, self.file_size)
    }

    pub fn get_cluster(&self) -> Cluster {
        //if ((self.first_cluster_high as u32) << 16) + (self.first_cluster_low as u32) == 0 {
            //println!("wtf how is this possible reeeeeeeeeeeeeeeeeeeeeeee");
            //println!("high cluster: {}, low_cluster: {}", self.first_cluster_high, self.first_cluster_low);
        //}
        Cluster::from(((self.first_cluster_high as u32) << 16) + (self.first_cluster_low as u32))
    }
    
    pub fn is_dir(&self) -> bool {
        self.attribute.is_dir()
    }
}

//const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    first_name: [u8; 10],
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
        use traits::Dir;
        for entry in self.entries()? {
            //dbg!(&entry);
            let file_name = String::from(entry.get_name());
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
    fn new_from_dir(root: &Dir<HANDLE>) -> io::Result<EntryIterator<HANDLE>> {
        //unimplemented!("EntryIterator::new_from_dir()")
        //dbg!(root);
        let vfat = root.vfat.clone();
        let mut buf: Vec<u8> = Vec::new();
        let num_read = vfat.lock(|fat: &mut VFat<HANDLE>| -> io::Result<usize> {
            //fat.read_chain(root.first_cluster, &mut buf).expect("failed to read chain in EntryIterator::new_from_dir()");
            fat.read_chain(root.first_cluster, &mut buf)
        });

        match num_read {
            Err(e) => {
                return Err(e);
            },
            Ok(_) => {
                let chain = unsafe {buf.cast::<VFatDirEntry>()};

                return Ok(EntryIterator{vfat, chain, index: 0});
            }
        }


        //let curr_entry = Entry::new(&chain[..32], vfat.clone());

    }
}

fn decode_name_from_slice(slice: &[u8]) -> io::Result<String> {
    let mut output = String::new();
    let iter = unsafe {decode_utf16(slice.cast::<u16>().iter().cloned())};
    for i in iter {
        match i {
            Ok('\u{0000}') => break,
            Ok('\u{ffff}') => break,
            Ok(c) => output.push(c),
            Err(e) => {return ioerr!(Other, "cannot decode utf16 string")}
        }
    }
    return Ok(output);
}

fn combine_string(vec: &Vec<String>) -> String {
    let mut output = String::new();
    for s in vec {
        output += s;
    }
    output
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;
    fn next(&mut self) -> Option<Self::Item> {
        //'outer: loop {
            let entry = unsafe {self.chain[self.index].unknown};
            if entry.attribute.is_lfn() {
                let mut vec: Vec<String> = Vec::new();
                let mut length = 0;
                'inner: loop {
                    let lfn_entry = unsafe {self.chain[self.index].long_filename};
                    if !lfn_entry.attribute.is_lfn() {
                        //return None;
                        break 'inner;
                    }
                    self.index += 1;
                    if lfn_entry.sequence_number > length {
                        length = lfn_entry.sequence_number;
                    }
                    let mut final_name = String::new();
                    match decode_name_from_slice(&lfn_entry.first_name) {
                        Ok(s) => final_name.push_str(s.as_str()),
                        Err(_) => {
                            return None;
                        }
                    };
                    match decode_name_from_slice(&lfn_entry.second_name) {
                        Ok(s) => final_name.push_str(s.as_str()),
                        Err(_) => {
                            return None;
                        }
                    };
                    match decode_name_from_slice(&lfn_entry.third_name) {
                        Ok(s) => final_name.push_str(s.as_str()),
                        Err(_) => {
                            return None;
                        }
                    };
                    if vec.len() < lfn_entry.sequence_number as usize {
                        vec.resize(lfn_entry.sequence_number as usize, String::from(""));
                    }
                    vec.insert(lfn_entry.sequence_number as usize, final_name);

                    //if lfn_entry.sequence_number & 0b01000000 != 0 {
                    //break;
                    //}
                }
                let lfn_name = combine_string(&vec);
                let reg_entry = unsafe {self.chain[self.index].regular};
                if reg_entry.get_metadata().is_end() {
                    return None;
                } else {
                    self.index += 1;

                    let metadata = reg_entry.get_metadata();
                    //if metadata.get_attribute().is_archive() {
                        //continue 'outer;
                    //}
                    let name = metadata.get_short_name();
                    println!("LFN: short name: {}, long name: {}, attribute: {:#x}, is_hidden: {}, is_system: {}, is_archive: {}, is_volume: {}", name, lfn_name, metadata.get_attribute().inner(), metadata.get_attribute().is_hidden(), metadata.get_attribute().is_system(), metadata.get_attribute().is_archive(), metadata.get_attribute().is_volume());
                    
                    if reg_entry.get_cluster().inner() == 0 {
                        println!("bruh wtf 0 on lfn");
                    }
                    return Some(Entry::from_regular_entry(reg_entry, self.vfat.clone(), lfn_name));
                }
            } else {
                let reg_entry = unsafe {self.chain[self.index].regular};
                self.index += 1;
                let metadata = reg_entry.get_metadata();
                //if metadata.get_attribute().is_archive() {
                //continue 'outer;
                //}
                if metadata.is_end() {
                    return None;
                }
                if reg_entry.get_cluster().inner() == 0 {
                    dbg!(reg_entry);
                }
                let name = metadata.get_short_name();
                println!("short name: {}, attribute: {:#x}, is_hidden: {}, is_system: {}, is_archive: {}, is_volume: {}"
                    , name, metadata.get_attribute().inner(), metadata.get_attribute().is_hidden(), metadata.get_attribute().is_system(), 
                    metadata.get_attribute().is_archive(), metadata.get_attribute().is_volume());
                return Some(Entry::from_regular_entry(reg_entry, self.vfat.clone(), name.to_string()));
            }
        }
    //}
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    // FIXME: Implement `trait::Dir` for `Dir`.
    type Entry = Entry<HANDLE>;
    type Iter = EntryIterator<HANDLE>;

    fn entries(&self) -> io::Result<Self::Iter> {
        //TODO: implement
        Ok(EntryIterator::new_from_dir(self)?)
    }
}
