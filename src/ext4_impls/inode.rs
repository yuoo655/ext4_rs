use bitflags::Flags;

use crate::ext4_defs::*;
use crate::prelude::*;
use crate::return_errno_with_message;
use crate::utils::bitmap::*;

impl Ext4 {
    pub fn get_bgid_of_inode(&self, inode_num: u32) -> u32 {
        inode_num / self.super_block.inodes_per_group()
    }

    pub fn inode_to_bgidx(&self, inode_num: u32) -> u32 {
        inode_num % self.super_block.inodes_per_group()
    }

    /// Get inode disk position.
    pub fn inode_disk_pos(&self, inode_num: u32) -> usize {
        let super_block = self.super_block;
        let inodes_per_group = super_block.inodes_per_group;
        let inode_size = super_block.inode_size as u64;
        let group = (inode_num - 1) / inodes_per_group;
        let index = (inode_num - 1) % inodes_per_group;
        let block_group =
            Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, group as usize);
        let inode_table_blk_num = block_group.get_inode_table_blk_num();
        let offset =
            inode_table_blk_num as usize * BLOCK_SIZE + index as usize * inode_size as usize;

        offset
    }

    /// Load the inode reference from the disk.
    pub fn get_inode_ref(&self, inode_num: u32) -> Ext4InodeRef {
        let offset = self.inode_disk_pos(inode_num);

        let mut ext4block = Block::load(self.block_device.clone(), offset);

        let inode: &mut Ext4Inode = ext4block.read_as_mut();

        Ext4InodeRef {
            inode_num: inode_num,
            inode: *inode,
        }
    }

    /// write back inode with checksum
    pub fn write_back_inode(&self, inode_ref: &mut Ext4InodeRef) {
        let inode_pos = self.inode_disk_pos(inode_ref.inode_num);

        // make sure self.super_block is up-to-date
        inode_ref
            .inode
            .set_inode_checksum(&self.super_block, inode_ref.inode_num);
        inode_ref
            .inode
            .sync_inode_to_disk(self.block_device.clone(), inode_pos);
    }

    /// write back inode with checksum
    pub fn write_back_inode_without_csum(&self, inode_ref: &Ext4InodeRef) {
        let inode_pos = self.inode_disk_pos(inode_ref.inode_num);

        inode_ref
            .inode
            .sync_inode_to_disk(self.block_device.clone(), inode_pos);
    }

    /// Get physical block id of a logical block.
    ///
    /// Params:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id
    pub fn get_pblock_idx(&self, inode_ref: &Ext4InodeRef, lblock: Ext4Lblk) -> Result<Ext4Fsblk> {
        let search_path = self.find_extent(&inode_ref, lblock);
        if let Ok(path) = search_path {
            // get the last path
            let path = path.path.last().unwrap();

            // get physical block id
            let fblock = path.pblock;

            return Ok(fblock);
        }

        return_errno_with_message!(Errno::EIO, "search extent fail");
    }

    /// Allocate a new block
    pub fn allocate_new_block(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk> {
        let mut super_block = self.super_block;
        let inodes_per_group = super_block.inodes_per_group();
        let bgid = (inode_ref.inode_num - 1) / inodes_per_group;
        let index = (inode_ref.inode_num - 1) % inodes_per_group;

        // load block group
        let mut block_group =
            Ext4BlockGroup::load_new(self.block_device.clone(), &super_block, bgid as usize);

        let block_bitmap_block = block_group.get_block_bitmap_block(&super_block);

        let mut block_bmap_raw_data = self
            .block_device
            .read_offset(block_bitmap_block as usize * BLOCK_SIZE);
        let mut data: &mut Vec<u8> = &mut block_bmap_raw_data;
        let mut rel_blk_idx = 0 as u32;

        ext4_bmap_bit_find_clr(data, index as u32, 0x8000, &mut rel_blk_idx);
        ext4_bmap_bit_set(&mut data, rel_blk_idx);

        block_group.set_block_group_balloc_bitmap_csum(&super_block, &data);
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
        let mut fb_cnt = block_group.get_free_blocks_count();
        fb_cnt -= 1;
        block_group.set_free_blocks_count(fb_cnt as u32);
        block_group.sync_to_disk_with_csum(self.block_device.clone(), bgid as usize, &super_block);

        Ok(rel_blk_idx as Ext4Fsblk)
    }

    /// Append a new block to the inode and update the extent tree.
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// iblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id of the new block
    pub fn append_inode_pblk(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        let mut newex: Ext4Extent = Ext4Extent::default();

        let new_block = self.balloc_alloc_block(inode_ref, None)?;

        newex.first_block = iblock;
        newex.store_pblock(new_block);
        newex.block_count = min(1, EXT_MAX_BLOCKS - iblock) as u16;

        self.insert_extent(inode_ref, &mut newex)?;

        // Update the inode size
        let mut inode_size = inode_ref.inode.size();
        inode_size += BLOCK_SIZE as u64;
        inode_ref.inode.set_size(inode_size);
        self.write_back_inode(inode_ref);

        Ok(new_block)
    }

    /// Append a new block to the inode and update the extent tree.From a specific bgid
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// bgid: Start bgid of free block search
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id of the new block
    pub fn append_inode_pblk_from(&self, inode_ref: &mut Ext4InodeRef, start_bgid: &mut u32) -> Result<Ext4Fsblk> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        let mut newex: Ext4Extent = Ext4Extent::default();

        let new_block = self.balloc_alloc_block_from(inode_ref, start_bgid)?;

        newex.first_block = iblock;
        newex.store_pblock(new_block);
        newex.block_count = min(1, EXT_MAX_BLOCKS - iblock) as u16;

        self.insert_extent(inode_ref, &mut newex)?;

        // Update the inode size
        let mut inode_size = inode_ref.inode.size();
        inode_size += BLOCK_SIZE as u64;
        inode_ref.inode.set_size(inode_size);
        self.write_back_inode(inode_ref);

        Ok(new_block)
    }

    /// Allocate a new inode
    ///
    /// Params:
    /// inode_mode: u16 - inode mode
    ///
    /// Returns:
    /// `Result<u32>` - inode number
    pub fn alloc_inode(&self, is_dir: bool) -> Result<u32> {
        // Allocate inode
        let inode_num = self.ialloc_alloc_inode(is_dir)?;

        Ok(inode_num)
    }

    pub fn correspond_inode_mode(&self, filetype: u8) -> u16 {
        let file_type = DirEntryType::from_bits(filetype).unwrap();
        match file_type {
            DirEntryType::EXT4_DE_REG_FILE => InodeFileType::S_IFREG.bits(),
            DirEntryType::EXT4_DE_DIR => InodeFileType::S_IFDIR.bits(),
            DirEntryType::EXT4_DE_SYMLINK => InodeFileType::S_IFLNK.bits(),
            DirEntryType::EXT4_DE_CHRDEV => InodeFileType::S_IFCHR.bits(),
            DirEntryType::EXT4_DE_BLKDEV => InodeFileType::S_IFBLK.bits(),
            DirEntryType::EXT4_DE_FIFO => InodeFileType::S_IFIFO.bits(),
            DirEntryType::EXT4_DE_SOCK => InodeFileType::S_IFSOCK.bits(),
            _ => {
                // FIXME: unsupported filetype
                InodeFileType::S_IFREG.bits()
            }
        }
    }
}
