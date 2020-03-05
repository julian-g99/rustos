use alloc::string::String;

use shim::io::{self, SeekFrom};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFat, VFatHandle, dir::VFatRegularDirEntry};

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
        //unimplemented!("File::read()")
        if self.metadata.get_file_size() == 0 {
            return Ok(0);
        }
        let num_bytes_per_cluster = self.vfat.lock(|handle: &mut VFat<HANDLE>| -> u64 {
            handle.bytes_per_cluster()
        });
        
        let mut num_clusters = self.metadata.get_file_size() as u64 / num_bytes_per_cluster;
        if self.metadata.get_file_size() as u64 % num_bytes_per_cluster != 0 {
            num_clusters += 1;
        }

        let mut vec = Vec::new();
        let result = self.vfat.lock(|handle: &mut VFat<HANDLE>| -> io::Result<usize> {
            //println!("file first cluster is: {}, file_name is: {}", self.first_cluster.inner(), self.get_name());
            handle.read_chain(self.first_cluster, &mut vec)
        });

        match result {
            Err(e) => return Err(e),
            Ok(_) => {
                let vec_slice = &vec[..num_clusters as usize * num_bytes_per_cluster as usize];
                if vec_slice.len() > buf.len() {
                    buf.copy_from_slice(&vec_slice[0..buf.len()]);
                    //println!("return greater than");
                    return Ok(buf.len());
                } else {
                    buf.copy_from_slice(vec_slice);
                    //println!("return less than");
                    return Ok(vec_slice.len());
                }
            }
        }
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
