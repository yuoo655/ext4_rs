use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
    pub fn dir_find_entry(
        &self,
        parent: &mut Ext4InodeRef,
        name: &str,
        result: &mut Ext4DirSearchResult,
    ) -> Result<usize> {
        let mut iblock = 0;
        let mut fblock: Ext4Fsblk = 0;

        let inode_size: u64 = parent.inode.size();
        let total_blocks: u64 = inode_size / BLOCK_SIZE as u64;

        while iblock < total_blocks {
            let search_path = self.find_extent(&parent, iblock as u32);

            if let Ok(path) = search_path {
                let path = path.path.last().unwrap();
                fblock = path.pblock;
                // load_block
                let mut ext4block =
                    Block::load(self.block_device.clone(), fblock as usize * BLOCK_SIZE);

                let r = self.dir_find_in_block(&mut ext4block, name);

                if r.is_ok() {
                    result.pblock_id = fblock as usize;
                    return Ok(EOK);
                }
            } else {
                return_errno_with_message!(Errno::ENOENT, "dir search fail")
            }

            iblock += 1
        }

        return_errno_with_message!(Errno::ENOENT, "dir search fail");
    }

    pub fn dir_find_in_block(&self, block: &Block, name: &str) -> Result<Ext4DirEntry> {
        let mut offset = 0;
        while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
            let de: Ext4DirEntry = block.read_offset_as(offset);
            if !de.unused() && de.compare_name(name) {
                return Ok(de);
            }
            offset += de.entry_len() as usize;
        }
        return_errno_with_message!(Errno::ENOENT, "dir find in block failed");
    }

    pub fn get_pblock_idx(&self, lblock: Ext4Lblk) -> Result<Ext4Fsblk> {
        return_errno_with_message!(Errno::ENOENT, "file not found");
    }
}
