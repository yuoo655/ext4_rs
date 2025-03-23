use crate::prelude::*;

use super::*;
use crate::utils::*;

#[cfg(feature = "blockcache")]
use spin::Mutex;

#[cfg(feature = "blockcache")]
#[derive(Clone)]
pub struct Ext4 {
    pub super_block: Ext4Superblock,

    pub block_device: Arc<dyn BlockDevice>,

    pub block_cache: Arc<Mutex<LruCache>>,
}

#[cfg(not(feature = "blockcache"))]
pub struct Ext4 {
    pub super_block: Ext4Superblock,

    pub block_device: Arc<dyn BlockDevice>,
}

impl Ext4 {
    pub fn read_offset(&self, offset: usize) -> Block {
        #[cfg(feature = "blockcache")]
        let block = Block::load(self.block_device.clone(), offset, self.block_cache.clone());

        #[cfg(not(feature = "blockcache"))]
        let block = Block::load(self.block_device, offset);

        block
    }

    pub fn block_group_sync_to_disk(&self, bg: &mut Ext4BlockGroup, bgid: usize) {
        bg.set_block_group_checksum(bgid as u32, &self.super_block);

        let dsc_cnt = BLOCK_SIZE / self.super_block.desc_size as usize;
        // let dsc_per_block = dsc_cnt;
        let dsc_id = bgid / dsc_cnt;
        // let first_meta_bg = super_block.first_meta_bg;
        let first_data_block = self.super_block.first_data_block;
        let block_id = first_data_block as usize + dsc_id + 1;
        let offset = (bgid % dsc_cnt) * self.super_block.desc_size as usize;
        let data = unsafe {
            core::slice::from_raw_parts(bg as *const _ as *const u8, size_of::<Ext4BlockGroup>())
        };

        let mut block = self.read_offset(block_id * BLOCK_SIZE + offset);
        block.write_offset(0, data, size_of::<Ext4BlockGroup>());
        
        block.sync_blk_to_disk(self.block_device.clone());
    }

    pub fn super_block_sync_to_disk(&self, super_block: &mut Ext4Superblock) {
        let data = unsafe {
            core::slice::from_raw_parts(
                super_block as *const _ as *const u8,
                size_of::<Ext4Superblock>(),
            )
        };
        let checksum = ext4_crc32c(EXT4_CRC32_INIT, data, 0x3fc);

        super_block.set_checksum(checksum);

        // let data = unsafe {
        //     core::slice::from_raw_parts(
        //         super_block as *const _ as *const u8,
        //         size_of::<Ext4Superblock>(),
        //     )
        // };
        let mut block = self.read_offset(SUPERBLOCK_OFFSET);
        let mut old_superblock:&mut Ext4Superblock = block.read_as_mut();
        *old_superblock = *super_block;

        // block.write_offset(0, data, size_of::<Ext4Superblock>());
        block.sync_blk_to_disk(self.block_device.clone());
        self.flush_cache();
    }

    pub fn flush_cache(&self) {
        let mut cache = self.block_cache.lock();
        for (offset, block) in cache.map.iter() {
            self.block_device.write_offset(*offset, &block.data);
        }
        cache.map.clear();
        cache.queue.clear();
    }
}
