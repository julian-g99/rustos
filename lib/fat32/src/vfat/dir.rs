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

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    vfat: HANDLE,
    first_cluster: Cluster,
    metadata: Metadata,
    name: String
}


impl<HANDLE: VFatHandle> Dir<HANDLE> {
    pub fn new(vfat: HANDLE, first_cluster: Cluster, metadata: Metadata) -> Self{
        let name = metadata.get_file_string_utf8().expect("dir name failed");
        Dir{vfat, first_cluster, metadata, name}
    }

    pub fn is_end(&self) -> bool {
        self.metadata.is_end()
    }

    pub fn get_name_utf8(&self) -> io::Result<&str> {
        //self.metadata.get_file_string_utf8(
        Ok(self.name.as_str())
    }
    
    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }
}

pub struct EntryIterator<HANDLE: VFatHandle> {
    //dir: Dir<HANDLE>,
    //curr_entry: Entry<HANDLE>
    chain: Vec<VFatDirEntry>,
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
        let mut buf: Vec<u8> = Vec::new();
        vfat.lock(|fat: &mut VFat<HANDLE>| {
            fat.read_chain(root.first_cluster, &mut buf).expect("failed to read chain in EntryIterator::new_from_dir()");
        });

        let chain = unsafe {buf.cast::<VFatDirEntry>()};

        //let curr_entry = Entry::new(&chain[..32], vfat.clone());

        EntryIterator{vfat, chain, index: 0}
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
    return Ok(output)
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
        //let entry = Entry::new(&(self.chain[self.index .. self.index + 32]), self.vfat.clone());
        //if entry.is_end() {
            //None
        //} else {
            //self.index += 32;
            //Some(entry)
        //}
        //let entry = unsafe {*(&self.chain[self.index .. self.index + 32] as *const VFatDirEntry)};
        //

        //let entry = unsafe {self.chain[self.index].unknown};
        //if entry.attribute.is_lfn() {
            //let vec: Vec<String> = Vec::new();
            //let mut length = 0;
            //loop {
                //let lfn_entry = unsafe {self.chain[self.index].long_filename};
                //self.index += 1;
                //if lfn_entry.sequence_number > length {
                    //length = lfn_entry.sequence_number;
                //}
                //let final_name = String::new();
                //match decode_name_from_slice(&lfn_entry.first_name) {
                    //Ok(s) => final_name.push_str(s.as_str()),
                    //Err(_) => return None
                //};
                //match decode_name_from_slice(&lfn_entry.first_name) {
                    //Ok(s) => final_name.push_str(s.as_str()),
                    //Err(_) => return None
                //};
                //match decode_name_from_slice(&lfn_entry.first_name) {
                    //Ok(s) => final_name.push_str(s.as_str()),
                    //Err(_) => return None
                //};
                //if vec.len() < lfn_entry.sequence_number as usize {
                    //vec.resize(lfn_entry.sequence_number as usize, String::from(""));
                //}
                //vec.insert(lfn_entry.sequence_number as usize, final_name);

                //if lfn_entry.sequence_number == 1 {
                    //break;
                //}
            //}
            //let lfn_name = combine_string(&vec);
            //return Some(Entry::new_from_file(File::new(handle, )))
        //} else {
            //let regular_entry = unsafe {entry.regular};
        //}
        unimplemented!("next function for EntryIterator")
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
