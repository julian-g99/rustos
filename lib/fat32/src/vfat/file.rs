use alloc::string::String;
use shim::ioerr;

use shim::io::{self, SeekFrom, Seek};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFat, VFatHandle, dir::VFatRegularDirEntry};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    vfat: HANDLE,
    first_cluster: Cluster,
    metadata: Metadata,
    name: String,
    cursor: u64,
}


impl<HANDLE: VFatHandle> File<HANDLE> {
    pub fn new(vfat: HANDLE, first_cluster: Cluster, metadata: Metadata, name: String) -> Self{
        File{vfat: vfat.clone(), first_cluster, metadata, name, cursor: 0}
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
        File{vfat, first_cluster, metadata, name, cursor: 0}
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
        if self.cursor >= self.metadata.get_file_size() as u64 {
            return Ok(0);
        }
        let mut vec = Vec::new();
        let result = self.vfat.lock(|handle: &mut VFat<HANDLE>| -> io::Result<usize> {
            handle.read_chain(self.first_cluster, &mut vec)
        });
        vec.resize(self.get_metadata().get_file_size() as usize, 0); //this should remove any trailing value
        vec = Vec::from(&vec[self.cursor as usize..]);

        match result {
            Err(e) => return Err(e),
            Ok(_) => {
                if vec.len() >= buf.len() {
                    buf.copy_from_slice(&vec[..buf.len()]);
                    //self.cursor += buf.len() as u64;
                    self.seek(SeekFrom::Current(buf.len() as i64))?;
                    return Ok(buf.len());
                } else {
                    let old_len = vec.len();
                    //vec.resize(buf.len(), 0);
                    buf[..vec.len()].copy_from_slice(vec.as_slice());
                    //buf.copy_from_slice(vec.as_slice());
                    //self.cursor += old_len as u64;
                    self.seek(SeekFrom::Current(old_len as i64))?;
                    return Ok(old_len);
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
        let position = match _pos {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::End(p) => self.metadata.get_file_size() as i64 - 1 + p,
            SeekFrom::Current(p) => self.cursor as i64 + p
        };

        if position < 0 {
            return ioerr!(InvalidInput, "seeking before the start of the file");
        } else if position > self.metadata.get_file_size() as i64 {
            return ioerr!(InvalidInput, "seeking beyond the end of the file");
        } else {
            self.cursor = position as u64;
            return Ok(position as u64);
        }
    }
}
