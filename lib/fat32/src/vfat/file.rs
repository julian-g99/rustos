use alloc::string::String;

use shim::io::{self, SeekFrom};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle, dir::VFatRegularDirEntry};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub first_cluster: Cluster,
    pub metadata: Metadata,
    name: String
}


impl<HANDLE: VFatHandle> File<HANDLE> {
    pub fn new(vfat: HANDLE, first_cluster: Cluster, metadata: Metadata, name: String) -> Self{
        File{vfat: vfat.clone(), first_cluster, metadata, name}
    }

    pub fn is_end(&self) -> bool {
        self.metadata.is_end()
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn from_regular_entry(handle: HANDLE, entry: VFatRegularDirEntry, name: String) -> Self {
        let vfat = handle.clone();
        //let first_cluster = Cluster::from(entry.get_cluster());
        let first_cluster = entry.get_cluster();
        let metadata = entry.get_metadata();
        File{vfat, first_cluster, metadata, name}
    }
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    fn sync(&mut self) -> io::Result<()> {
        unimplemented!("File::sync()")
    }

    fn size(&self) -> u64 {
        return self.metadata.get_file_size() as u64
    }
}

impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, buf: &mut[u8]) -> io::Result<usize> {
        unimplemented!("File::read()")
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unimplemented!("File::write()")
    }
    
    fn flush(&mut self) -> io::Result<()> {
        unimplemented!("File::flush()")
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        unimplemented!("File::seek()")
    }
}
