use crate::prelude::*;
use crate::BLOCK_SIZE;
use crate::BlockDevice;

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
        block_device.write_offset(block_id * BLOCK_SIZE, &self.block_data);
    }
}