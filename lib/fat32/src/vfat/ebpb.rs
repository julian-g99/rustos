use core::fmt;
use shim::const_assert_size;

use crate::traits::BlockDevice;
use crate::vfat::Error;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct BiosParameterBlock {
    first_three: [u8; 3],
    oem_id: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub num_reserved_sectors: u16,
    pub num_fats: u8,
    num_dir_entries: u16,
    num_logical_sectors: u16,
    fat_id: u8,
    num_sectors_per_fat: u16, //NOTE: should be 0 for FAT 32
    num_sectors_per_track: u16,
    num_heads: u16,
    num_hidden_sectors: u32,
    total_logical_sectors: u32,
    pub sectors_per_fat: u32,
    flags: u16,
    fat_version_number: u16,
    pub rootdir_cluster: u32,
    fs_info_sector: u16,
    backup_boot_sector: u16,
    reserved: [u8; 12],
    drive_number: u8,
    flag_windows: u8,
    signature: u8,
    volume_id: u32,
    volumne_label_string: [u8; 11],
    system_identifier_string: [u8; 8],
    boot_code:[u8; 420],
    boot_signature: u16
}

//const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut arr = [0u8; 512];
        let buf = &mut arr[..];
        match device.read_sector(sector, buf) {
            Err(e) => {
                return Err(Error::Io(e));
            },
            Ok(_) => {
                let ebpb = unsafe{ *(buf.as_mut_ptr() as *mut BiosParameterBlock) };
                if ebpb.boot_signature != 0xAA55 {
                    dbg!(ebpb.signature);
                    return Err(Error::BadSignature);
                }
                return Ok(ebpb);
            }
        }
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //unimplemented!("BiosParameterBlock::fmt()")
        write!(f, "num bytes per sector: {:?}, num sectors per cluster: {:?}, \
            num fats:{:?}, total logical directories: {:?}, sectors per fat: {:?}, signature: {:?}, \
            boot parition signature: {:?}", &{self.bytes_per_sector}, self.sectors_per_cluster,
            self.num_fats, &{self.total_logical_sectors}, &{self.sectors_per_fat}, self.signature, &{self.boot_signature})
    }
}
