use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle, Cluster, dir::VFatDirEntry, dir::VFatUnknownDirEntry, dir::VFatRegularDirEntry};
use core::fmt;
use shim::io;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

impl<HANDLE: VFatHandle> Entry<HANDLE> {
    pub fn new(slice: &[u8], handle: HANDLE) -> Self {
        assert_eq!(slice.len(), 32, "slice given to Entry::new() isn't length 32");
        let vfat = handle.clone();
        let first_cluster = Cluster::first_cluster_of_entry(slice);
        let metadata = Metadata::from(slice);

        if metadata.get_attribute().is_dir() {
            Entry::Dir(Dir::new(vfat, first_cluster, metadata))
        } else {
            Entry::File(File::new(vfat, first_cluster, metadata))
        }
        
        //match arr[11] {
            //0x10 => {
                ////TODO: parse as a directory
            //},
            //0x0F => {
                ////TODO: parse as a lfn
            //},
            //_ => {
                ////TODO: parse asas normal file
            //},
        //}
    }


    pub fn new_from_dir(dir: Dir<HANDLE>) -> Self {
        Entry::Dir(dir)
    }

    pub fn new_from_file(file: File<HANDLE>) -> Self {
        Entry::File(file)
    }
}

impl<HANDLE: VFatHandle> Entry<HANDLE> {
    pub fn is_end(&self) -> bool {
        match self {
            Entry::Dir(d) => {
                d.is_end()
            },
            Entry::File(f) => {
                f.is_end()
            }
        }
    }

    pub fn get_name_utf8(&self) -> io::Result<&str> {
        match self {
            Entry::Dir(d) => {
                d.get_name_utf8()
            },
            Entry::File(f) => {
                f.get_name_utf8()
            }
        }
    }
}

// TODO: Implement any useful helper methods on `Entry`.

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    fn name(&self) -> &str {
        match self {
            Entry::Dir(d) => d.get_name_utf8().unwrap(),
            Entry::File(f) => f.get_name_utf8().unwrap()
        }
    }

    fn metadata(&self) -> &Self::Metadata {
        match self {
            Entry::Dir(d) => d.get_metadata(),
            Entry::File(f) => f.get_metadata(),
        }
    }

    fn as_file(&self) -> Option<&File<HANDLE>> {
        match self {
            Entry::File(f) => Some(&f),
            _ => None
        }
    }

    fn as_dir(&self) -> Option<&Dir<HANDLE>> {
        match self {
            Entry::Dir(d) => Some(&d),
            _ => None
        }
    }

    fn into_file(self) -> Option<File<HANDLE>> {
        match self {
            Entry::File(f) => Some(f),
            _ => None
        }
    }

    fn into_dir(self) -> Option<Dir<HANDLE>> {
        match self {
            Entry::Dir(d) => Some(d),
            _ => None
        }
    }
}
