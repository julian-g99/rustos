use core::fmt;

use alloc::string::String;

use crate::traits;
use crate::util::SliceExt;
//use std::convert::TryInto;
use core::convert::TryInto;
//use std::str::from_utf8;
use shim::io;
use shim::ioerr;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

impl From::<&[u8]> for Date {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 2);
        Date(((slice[1] as u16) << 8) + slice[0] as u16)
    }
}

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

impl From::<&[u8]> for Time {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 2);
        //dbg!(slice);
        Time(((slice[1] as u16) << 8) + slice[0] as u16)
    }
}

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

impl From::<u8> for Attributes {
    fn from(val: u8) -> Self {
        Attributes(val)
    }
}

impl Attributes {
    pub fn is_dir(&self) -> bool {
        //self.0 == 0x10
        self.0 & 0x10 != 0
    }

    
    pub fn inner(&self) -> u8 {
        self.0
    }

    pub fn is_hidden(&self) -> bool {
        self.0 & 0x02 != 0
        //self.0 == 0x02
    }

    pub fn is_archive(&self) -> bool {
        self.0 & 0x20 != 0
        //self.0 == 0x20
    }
    
    pub fn is_system(&self) -> bool {
        //self.0 == 0x04
        self.0 & 0x04 != 0
    }

    pub fn is_lfn(&self) -> bool {
        self.0 == 0x0F
        //self.0 & 0x0F != 0
    }

    pub fn is_volume(&self) -> bool {
        self.0 & 0x08 != 0
    }
}

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    date: Date,
    time: Time,
}

impl Timestamp {
    pub fn new(date: Date, time: Time) -> Self {
        Timestamp{date, time}
    }

    fn new_from_slices(date: &[u8], time: &[u8]) -> Self {
        assert_eq!(date.len(), 2, "date given to new_from_slice length isn't 2");
        assert_eq!(time.len(), 2, "time given to new_from_slice length isn't 2");

        let date = Date::from(date);
        let time = Time::from(time);

        Timestamp{date, time}
    }
}

/// Metadata for a directory entry.
//#[derive(Default, Debug, Clone)]
#[derive(Default, Clone)]
pub struct Metadata {
    //file_name: [u8; 8],
    //file_extension: [u8; 3],
    id: u8,
    file_name: [u8; 8],
    file_extension: [u8; 3],
    //short_name: Option<String>,
    short_name: String,
    attribute: Attributes,
    //reserved_for_windows: u8,
    //create_time_10th_second: u8,
    creation_time: Timestamp,
    last_access_date: Timestamp,
    //first_cluster_high_16: u16,
    last_modification_date: Timestamp,
    //first_cluster_low_16: u16,
    file_size: u32
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //write!(f, "id: {}, file name: {}, file extension: {}, attributes: {:?}, creation time: {:?}, last_access_date: {:?}, last_modification_date: {:?}, file_size: {}",
            //self.id, std::str::from_utf8(&self.file_name), std::str::from_utf8(&self.file_extension), self.attribute, self.creation_time, self.last_access_date, self.last_modification_date, self.file_size)
        write!(f, "id: {}, file name: {:?}, file_size: {}",
            self.id, self.short_name, self.file_size)
    }
}

fn u8_to_u32 (slice: &[u8]) -> u32{
    assert_eq!(slice.len(), 4);
    let mut output = 0u32;
    for i in 0..4 {
        output += (slice[i] as u32) << (i * 8);
    }
    output
    
    //let new_slice: &[u32] = unsafe{ slice.cast() };
    //return new_slice[0];
}

fn u8_to_u64 (slice: &[u8]) -> u64{
    assert_eq!(slice.len(), 8);
    let mut output = 0u64;
    for i in 0..8 {
        output += (slice[i] as u64) << (i * 8);
    }
    output

    //let new_slice: &[u64] = unsafe { slice.cast() };
    //return new_slice[0];
}

fn combine_to_short_name(file_name: &[u8], file_extension: &[u8]) -> String {
    assert_eq!(file_name.len(), 8);
    assert_eq!(file_extension.len(), 3);

    let mut name = String::new();
    let mut extension = String::new();
    for c in file_name {
        if *c == 0x00 || *c == 0x20 {
            break;
        } else {
            name.push(*c as char);
        }
    }
    for c in file_extension {
        if *c == 0x00 || *c == 0x20 {
            break;
        } else {
            extension.push(*c as char);
        }
    }

    if extension.len() != 0 {
        name + "." + extension.as_str()
    } else {
        name
    }
}

impl From::<&[u8]> for Metadata {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 32, "vector given to Metadata::from() isn't length 32");
        let id = slice[0];
        let file_name: [u8; 8] = slice[..8].try_into().expect("slice with incorrect length");
        let file_extension: [u8; 3] = slice[8..11].try_into().expect("slice with incorrect length");
        let attribute = Attributes::from(slice[11]);
        //let mut short_name = None;
        //if !attribute.is_lfn() {
            //short_name = Some(combine_to_short_name(&slice[..8], &slice[8..11]));
        //}
        let mut short_name = String::new();
        if !attribute.is_lfn() {
            short_name = combine_to_short_name(&slice[..8], &slice[8..11]);
        }
        let creation_time = Timestamp::new_from_slices(&slice[14..16], &slice[16..18]);
        let last_access_date = Timestamp::new_from_slices(&slice[18..20], &[0, 0]);
        let last_modification_date = Timestamp::new_from_slices(&slice[22..24], &slice[24..26]);
        let file_size = u8_to_u32(&slice[28..]);

        Metadata{id, file_name, file_extension, short_name, attribute, creation_time, last_access_date, last_modification_date, file_size}
    }
}


impl Metadata {
    pub fn new(name: &[u8], extension: &[u8], attribute: Attributes, creation_time: Timestamp, last_access_date: Timestamp,
        last_modification_date: Timestamp, file_size: u32) -> Self {
        assert_eq!(name.len(), 8, "name given to Metadata::new() isn't length 8");
        assert_eq!(extension.len(), 3, "extension given to Metadata::new() isn't length 3");
        let file_name: [u8; 8] = name.try_into().expect("slice with incorrect length");
        let file_extension: [u8; 3] = extension.try_into().expect("slice with incorrect length");
        let id = file_name[0];
        //let mut short_name = None;
        //if !attribute.is_lfn() {
            //short_name = Some(combine_to_short_name(name, extension));
        //}
        let mut short_name = String::new();
        if !attribute.is_lfn() {
            short_name = combine_to_short_name(name, extension);
        }
        Metadata {id, file_name, file_extension, short_name, attribute, creation_time, last_access_date, last_modification_date, file_size}
    }


    pub fn get_attribute(&self) -> Attributes {
        self.attribute
    }

    pub fn is_end(&self) -> bool {
        self.id == 0x00
    }

    pub fn is_deleted_or_unused(&self) -> bool {
        self.id == 0xE5
    }

    pub fn get_short_name(&self) -> &String {
        &self.short_name
    }

    pub fn get_file_size(&self) -> u32 {
        self.file_size
    }
}

/// Gets the value at bit range starting at `start` and ending at `end` (both indices are inclusive)
/// It is the responsibility of the caller to make sure that end is always at least as large as
/// start
fn get_bit_range(num: u16, start: u16, end: u16) -> u8 {
    assert!(end >= start);
    //let mask = (1 << (end - start + 1)) << start;
    let mask = ((1 << (end - start + 1)) - 1) << start;
    ((num & mask) >> start) as u8
}

// FIXME: Implement `traits::Timestamp` for `Timestamp`.
impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        1980 + get_bit_range(self.date.0, 9, 15) as usize
    }

    fn month(&self) -> u8 {
        get_bit_range(self.date.0, 5, 8)
    }

    fn day(&self) -> u8 {
        get_bit_range(self.date.0, 0, 4)
    }

    fn hour(&self) -> u8 {
        get_bit_range(self.time.0, 11, 15)
    }

    fn minute(&self) -> u8 {
        get_bit_range(self.time.0, 5, 10)
    }

    fn second(&self) -> u8 {
        get_bit_range(self.time.0, 0, 4) * 2
    }
}

// FIXME: Implement `traits::Metadata` for `Metadata`.
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attribute.0 == 0x01
    }

    fn hidden(&self) -> bool {
        //self.attribute.0 == 0x02
        self.attribute.is_hidden()
    }

    fn created(&self) -> Self::Timestamp {
        self.creation_time
    }

    fn accessed(&self) -> Self::Timestamp {
        self.last_access_date
    }

    fn modified(&self) -> Self::Timestamp {
        self.last_modification_date
    }
}

//fn concat_number(high: u16, low: u16) -> u32 {
    //(high as u32) << 16 + (low as u32)
//}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::traits::Timestamp;
        write!(f, "Year: {}, Month: {}, Day: {},  Hour: {}, Minute: {}, Second: {}",
            self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second())
    }
}

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.
impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //write!(f, "file name: {:?}, extension: {:?}, creation time: {:?}, \
            //last access date: {:?}, first cluster: {:?}, last modified: {:?}, file size: {:?}",
            //self.file_name, self.file_extension, self.creation_time,
            //self.last_access_date, concat_number(self.first_cluster_high_16, self.first_cluster_low_16), self.last_modification_date, self.file_size)
        write!(f, "creation time: {:?}, \
            last access date: {:?}, last modified: {:?}, file size: {:?}",
            self.creation_time,
            self.last_access_date, self.last_modification_date, self.file_size)
    }
}
