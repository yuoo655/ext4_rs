use crate::prelude::*;
use crate::BLOCK_SIZE;
use crate::BlockDevice;
use crate::extent::*;


#[derive(Debug)]
// A single block descriptor
pub struct Ext4Block<'a> {
    pub logical_block_id: u32, // 逻辑块号

    // disk block id
    pub disk_block_id: u64,

    // size BLOCK_SIZE
    pub block_data: &'a mut Vec<u8>,

    pub dirty: bool,
}

impl <'a>Ext4Block<'a>{
    pub fn sync_blk_to_disk(&self, block_device: Arc<dyn BlockDevice>  ){
        let block_id = self.disk_block_id as usize;

        debug!("write block id {:x?}", block_id);
        block_device.write_offset(block_id * BLOCK_SIZE, &self.block_data);
    }
    // 将块数据的指针转换为 Ext4ExtentHeader 指针
    pub fn ext_block_hdr(&self) -> *mut Ext4ExtentHeader {
        self.block_data.as_ptr() as *mut Ext4ExtentHeader
    }
}


// impl <'a>Default for Ext4Block<'a> {
//     fn default() -> Self {
//         Self {
//             logical_block_id: 0,
//             disk_block_id: 0,
//             block_data: &mut Vec::new(),
//             dirty: false,
//         }
//     }
// }