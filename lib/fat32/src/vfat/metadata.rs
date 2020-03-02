use core::fmt;

use alloc::string::String;

use crate::traits;
use std::convert::TryInto;
use std::str::from_utf8;
use shim::io;
use shim::ioerr;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

impl From::<&[u8]> for Date {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 2);
        Date((slice[1] as u16) << 8 + slice[0] as u16)
    }
}

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

impl From::<&[u8]> for Time {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 2);
        dbg!(slice);
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
        self.0 == 0x10
    }
}

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    date: Date,
    time: Time,
}

impl Timestamp {
    fn new(date: Date, time: Time) -> Self {
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
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    //file_name: [u8; 8],
    //file_extension: [u8; 3],
    id: u8,
    file_name: [u8; 8],
    file_extension: [u8; 3],
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

fn u8_to_u32 (slice: &[u8]) -> u32{
    assert_eq!(slice.len(), 4);
    let mut output = 0u32;
    for i in 0..4 {
        output += (slice[i] as u32) << (i * 8);
    }
    output
}

fn u8_to_u64 (slice: &[u8]) -> u64{
    assert_eq!(slice.len(), 8);
    let mut output = 0u64;
    for i in 0..8 {
        output += (slice[i] as u64) << (i * 8);
    }
    output
}

impl From::<&[u8]> for Metadata {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), 32, "vector given to Metadata::from() isn't length 32");
        let id = slice[0];
        //let file_name = u8_to_u64(&slice[..8]);
        let file_name: [u8; 8] = slice[..8].try_into().expect("slice with incorrect length");
        let file_extension: [u8; 3] = slice[8..11].try_into().expect("slice with incorrect length");
        let attribute = Attributes::from(slice[11]);
        let creation_time = Timestamp::new_from_slices(&slice[14..16], &slice[16..18]);
        let last_access_date = Timestamp::new_from_slices(&slice[18..20], &[0, 0]);
        let last_modification_date = Timestamp::new_from_slices(&slice[22..24], &slice[24..26]);
        let file_size = u8_to_u32(&slice[28..]);

        Metadata{id, file_name, file_extension, attribute, creation_time, last_access_date, last_modification_date, file_size}
    }
}

impl Metadata {
    pub fn get_attribute(&self) -> Attributes {
        self.attribute
    }

    pub fn is_end(&self) -> bool {
        self.id == 0x00
    }

    pub fn get_file_string_utf8(&self) -> io::Result<String> {
        let name = match from_utf8(&self.file_name) {
            Err(_) => {
                return ioerr!(Other, "parsing file name (regular) to string failed");
            },
            Ok(s) => s
        };
        let extension = match from_utf8(&self.file_extension) {
            Err(_) => {
                return ioerr!(Other, "parsing file extension (regular) to string failed");
            },
            Ok(s) => s
        };

        Ok(format!("{}.{}", name, extension))
    }
}

/// Gets the value at bit range starting at `start` and ending at `end` (both indices are inclusive)
/// It is the responsibility of the caller to make sure that end is always at least as large as
/// start
fn get_bit_range(num: u16, start: u16, end: u16) -> u8 {
    assert!(end >= start);
    let mask = (1 << (end - start + 1)) << start;
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
        get_bit_range(self.time.0, 0, 4)
    }
}

// FIXME: Implement `traits::Metadata` for `Metadata`.
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attribute.0 == 0x01
    }

    fn hidden(&self) -> bool {
        self.attribute.0 == 0x02
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
