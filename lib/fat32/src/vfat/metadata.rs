use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    //file_name: [u8; 8],
    //file_extension: [u8; 3],
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
