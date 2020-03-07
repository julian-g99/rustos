use crate::util::SliceExt;


#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Hash)]
pub struct Cluster(u32); //TODO: should I do this?

impl From<u32> for Cluster {
    fn from(raw_num: u32) -> Cluster {
        Cluster(raw_num & !(0xF << 28))
    }
}

fn u8_to_u16(slice: &[u8]) -> u16 {
    assert_eq!(slice.len(), 2);
    let mut output = 0u16;
    for i in 0..2 {
        output += (slice[i] as u16) << (i * 8);
    }

    output
}

// TODO: Implement any useful helper methods on `Cluster`.
impl Cluster {
    pub fn get_start_sector(&self, sectors_per_cluster: u64, data_start_sector: u64) -> u64 {
        (self.0 as u64 - 2) * sectors_per_cluster + data_start_sector
    }

    pub fn inner(&self) -> u32 {
        self.0
    }

    pub fn first_cluster_of_entry(slice: &[u8]) -> Self{
        //assert_eq!(slice.len(), 32);
        //let high: &[u16] = unsafe {&slice[20..22].cast()};
        //let low: &[u16] = unsafe {&slice[26..28].cast()};
        //let high_val = high[0] as u32;
        //let low_val = low[0] as u32;
        let high_val = u8_to_u16(&slice[20..22]) as u32;
        let low_val = u8_to_u16(&slice[26..28]) as u32;
        Cluster::from((high_val << 16) + low_val)
        
        //let low = &slice[26..28];
        //let high = &slice[20..22];

        //let high_val = ((high[1] as u32) << 8) + (high[0] as u32);
        //let low_val = ((low[1] as u32) << 8 )+ (low[0] as u32);
        //Cluster::from((high_val << 16) + low_val)

    }
}
