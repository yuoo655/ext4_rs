use crate::ext4_defs::*;
use crate::prelude::*;
use crate::return_errno_with_message;

impl Ext4 {
    /// Get inode disk position.
    pub fn inode_disk_pos(&self, inode: u32) -> usize {
        let super_block = self.super_block;
        let inodes_per_group = super_block.inodes_per_group;
        let inode_size = super_block.inode_size as u64;
        let group = (inode - 1) / inodes_per_group;
        let index = (inode - 1) % inodes_per_group;
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

    // pub fn find_block(&self, inode_ref: &Ext4InodeRef, iblock: &mut Ext4Lblk) -> Ext4Fsblk {
    //     todo!()
    // }

    // pub fn dir_find_entry(&self, parent: &mut Ext4InodeRef, name: &str) -> Result<Ext4DirEntry> {
    //     let inode_size: u32 = parent.inode.size;
    //     let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

    //     let mut iblock = 0;
    //     while iblock < total_blocks {
    //         let fblock = self.find_block(&parent, &mut iblock);
    //     }
    //     return_errno_with_message!(Errno::ENOENT, "file not found");
    // }
}
