use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle, Cluster};
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

    pub fn get_name_utf8(&self) -> io::Result<String> {
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
        unimplemented!("Entry::name()")
    }

    fn metadata(&self) -> &Self::Metadata {
        unimplemented!("Entry::metadata()")
    }

    fn as_file(&self) -> Option<&File<HANDLE>> {
        unimplemented!("Entry::as_file()")
    }

    fn as_dir(&self) -> Option<&Dir<HANDLE>> {
        unimplemented!("Entry::as_dir()")
    }

    fn into_file(self) -> Option<File<HANDLE>> {
        unimplemented!("Entry::into_file()")
    }

    fn into_dir(self) -> Option<Dir<HANDLE>> {
        unimplemented!("Entry::into_dir()")
    }
}
