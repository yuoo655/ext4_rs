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
    
    /// Get physical block id of a logical block.
    /// 
    /// Parms:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    /// 
    /// Returns:
    /// Result<Ext4Fsblk> - physical block id
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
}
