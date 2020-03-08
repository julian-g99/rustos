use alloc::string::String;
use alloc::vec::Vec;

use core::fmt;

use shim::const_assert_size;
use core::char::decode_utf16;
use crate::alloc::string::ToString;
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
        Cluster::from(((self.first_cluster_high as u32) << 16) + (self.first_cluster_low as u32))
    }
    
    pub fn is_dir(&self) -> bool {
        self.attribute.is_dir()
    }

    pub fn is_deleted_or_unused(&self) -> bool {
        self.file_name[0] == 0xE5
    }
}

//const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    first_name: [u16; 5],
    attribute: Attributes,
    file_type: u8,
    checksum: u8,
    second_name: [u16; 6],
    reserved: u16,
    third_name: [u16; 2]
}

impl VFatLfnDirEntry {
    pub fn is_end(&self) -> bool {
        self.sequence_number == 0
    }

    pub fn is_deleted_or_unused(&self) -> bool {
        self.sequence_number == 0xE5
    }

    pub fn is_first_lfn(&self) -> bool {
        self.sequence_number & (1 << 5) == 0
    }
}

impl VFatLfnDirEntry {
    fn get_lfn(&self) -> Option<String> {
        let mut final_name = String::new();
        match decode_name_from_slice(&{self.first_name}) {
            Ok(s) => final_name.push_str(s.as_str()),
            Err(_) => {
                return None;
            }
        };
        match decode_name_from_slice(&{self.second_name}) {
            Ok(s) => final_name.push_str(s.as_str()),
            Err(_) => {
                return None;
            }
        };
        match decode_name_from_slice(&{self.third_name}) {
            Ok(s) => final_name.push_str(s.as_str()),
            Err(_) => {
                return None;
            }
        };
        Some(final_name)
    }
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

fn decode_name_from_slice(slice: &[u16]) -> io::Result<String> {
    let mut output = String::new();
    let iter = decode_utf16(slice.iter().cloned());
    for i in iter {
        match i {
            Ok('\u{0000}') => break,
            Ok('\u{ffff}') => break,
            Ok(c) => output.push(c),
            Err(_) => return ioerr!(Other, "cannot decode utf16 string")
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
        'outer: loop {
            let entry = unsafe {self.chain[self.index].unknown};
            if entry.attribute.is_lfn() {
                let mut vec: Vec<String> = Vec::new();
                'inner1: loop {
                    let lfn_entry = unsafe {self.chain[self.index].long_filename};
                    if lfn_entry.is_end() {
                        return None;
                    } else if lfn_entry.is_deleted_or_unused() {
                        self.index += 1;
                        continue 'outer;
                    }
                    if !lfn_entry.attribute.is_lfn() {
                        break 'inner1;
                    }
                    self.index += 1;
                    let final_name = match lfn_entry.get_lfn() {
                        Some(s) => s,
                        None => return None
                    };
                    if vec.len() < lfn_entry.sequence_number as usize {
                        vec.resize(lfn_entry.sequence_number as usize, String::from(""));
                    }
                    vec.insert(lfn_entry.sequence_number as usize, final_name);
                }
                'inner2: loop {
                    let reg_entry = unsafe {self.chain[self.index].regular};
                    if reg_entry.get_metadata().is_end() {
                        return None;
                    } else if reg_entry.is_deleted_or_unused() {
                        self.index += 1;
                        continue 'inner2;
                        //continue 'outer;
                    } else {
                        self.index += 1;
                        //println!("SFN of LFN: {}", reg_entry.get_metadata().get_short_name());
                        let lfn_name = combine_string(&vec);
                        //println!("LFN: {}", lfn_name);
                        //println!("LFN name: {}, first_cluster: {}, size: {}", lfn_name, reg_entry.get_cluster().inner(), reg_entry.file_size);
                        let result = Entry::from_regular_entry(reg_entry, self.vfat.clone(), lfn_name);
                        return Some(result);
                    }
                }
            } else {
                let reg_entry = unsafe {self.chain[self.index].regular};
                let metadata = reg_entry.get_metadata();
                if metadata.is_end() {
                    return None;
                }
                if reg_entry.is_deleted_or_unused() {
                    self.index += 1;
                    continue 'outer;
                }
                //println!("SFN: {}", reg_entry.get_metadata().get_short_name());
                self.index += 1;
                //println!("SFN name: {}, first_cluster: {}, size: {}", metadata.get_short_name(), reg_entry.get_cluster().inner(), reg_entry.file_size);
                let name = metadata.get_short_name();
                return Some(Entry::from_regular_entry(reg_entry, self.vfat.clone(), name.to_string()));
            }
        }
    }
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
