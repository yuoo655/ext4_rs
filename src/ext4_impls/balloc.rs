use crate::ext4_defs::*;
use crate::prelude::*;
use crate::return_errno_with_message;
use crate::utils::bitmap::*;

impl Ext4 {
    /// Allocate a new block
    pub fn balloc_alloc_block(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk> {
        let mut super_block = self.super_block;
        let inodes_per_group = super_block.inodes_per_group();
        let bgid = (inode_ref.inode_num - 1) / inodes_per_group;
        let index = (inode_ref.inode_num - 1) % inodes_per_group;

        // load block group
        let mut bg =
            Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

        let block_bitmap_block = bg.get_block_bitmap_block(&super_block);

        let mut block_bmap_raw_data = self
            .block_device
            .read_offset(block_bitmap_block as usize * BLOCK_SIZE);
        let mut data: &mut Vec<u8> = &mut block_bmap_raw_data;
        let mut rel_blk_idx = 0 as u32;

        ext4_bmap_bit_find_clr(data, index as u32, 0x8000, &mut rel_blk_idx);
        ext4_bmap_bit_set(&mut data, rel_blk_idx);

        bg.set_block_group_balloc_bitmap_csum(&super_block, &data);
        self.block_device
            .write_offset(block_bitmap_block as usize * BLOCK_SIZE, &data);

        /* Update superblock free blocks count */
        let mut super_blk_free_blocks = super_block.free_blocks_count();
        super_blk_free_blocks -= 1;
        super_block.set_free_blocks_count(super_blk_free_blocks);
        super_block.sync_to_disk_with_csum(self.block_device.clone());

        /* Update inode blocks (different block size!) count */
        let mut inode_blocks = inode_ref.inode.blocks_count();
        inode_blocks += (BLOCK_SIZE / EXT4_INODE_BLOCK_SIZE) as u64;
        inode_ref.inode.set_blocks_count(inode_blocks);
        self.write_back_inode(inode_ref);

        /* Update block group free blocks count */
        let mut fb_cnt = bg.get_free_blocks_count();
        fb_cnt -= 1;
        bg.set_free_blocks_count(fb_cnt as u32);
        bg.sync_to_disk_with_csum(self.block_device.clone(), bgid as usize, &super_block);

        Ok(rel_blk_idx as Ext4Fsblk)
    }

    #[allow(unused)]
    pub fn balloc_free_blocks(
        &mut self,
        inode_ref: &mut Ext4InodeRef,
        start: Ext4Fsblk,
        count: u32,
    ) {
        let mut count = count as usize;
        let mut start = start;

        let mut super_block = self.super_block;

        let blocks_per_group = super_block.blocks_per_group();

        let bgid = start / blocks_per_group as u64;

        let mut bg_first = start / blocks_per_group as u64;
        let mut bg_last = (start + count as u64 - 1) / blocks_per_group as u64;

        while bg_first <= bg_last {
            let idx_in_bg = start % blocks_per_group as u64;

            let mut bg =
                Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

            let block_bitmap_block = bg.get_block_bitmap_block(&super_block);
            let mut raw_data = self
                .block_device
                .read_offset(block_bitmap_block as usize * BLOCK_SIZE);
            let mut data: &mut Vec<u8> = &mut raw_data;

            let mut free_cnt = BLOCK_SIZE * 8 - idx_in_bg as usize;

            if count as usize > free_cnt {
            } else {
                free_cnt = count as usize;
            }

            ext4_bmap_bits_free(&mut data, idx_in_bg as u32, free_cnt as u32);

            count -= free_cnt;
            start += free_cnt as u64;

            bg.set_block_group_balloc_bitmap_csum(&super_block, &data);
            self.block_device
                .write_offset(block_bitmap_block as usize * BLOCK_SIZE, &data);

            /* Update superblock free blocks count */
            let mut super_blk_free_blocks = super_block.free_blocks_count();

            super_blk_free_blocks += free_cnt as u64;
            super_block.set_free_blocks_count(super_blk_free_blocks);
            super_block.sync_to_disk_with_csum(self.block_device.clone());

            /* Update inode blocks (different block size!) count */
            let mut inode_blocks = inode_ref.inode.blocks_count();

            inode_blocks -= (free_cnt as usize * (BLOCK_SIZE / EXT4_INODE_BLOCK_SIZE)) as u64;
            inode_ref.inode.set_blocks_count(inode_blocks);
            self.write_back_inode(inode_ref);

            /* Update block group free blocks count */
            let mut fb_cnt = bg.get_free_blocks_count();
            fb_cnt += free_cnt as u64;
            bg.set_free_blocks_count(fb_cnt as u32);
            bg.sync_to_disk_with_csum(self.block_device.clone(), bgid as usize, &super_block);

            bg_first += 1;
        }
    }
}
