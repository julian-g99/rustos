use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
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
