use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
    /// Link a child inode to a parent directory
    ///
    /// Params:
    /// parent: &mut Ext4InodeRef - parent directory inode reference
    /// child: &mut Ext4InodeRef - child inode reference
    /// name: &str - name of the child inode
    ///
    /// Returns:
    /// Result<usize> - status of the operation
    ///
    pub fn link(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        name: &str,
    ) -> Result<usize> {
        // Add a directory entry in the parent directory pointing to the child inode

        // at this point should insert to existing block
        self.dir_add_entry(parent, child, name)?;
        self.write_back_inode_without_csum(parent);

        // If this is the first link. add '.' and '..' entries
        if child.inode.is_dir() {
            // let child_ref = child.clone();
            let new_child_ref = Ext4InodeRef {
                inode_num: child.inode_num,
                inode: child.inode.clone(),
            };

            // at this point child need a new block
            self.dir_add_entry(child, &new_child_ref, ".")?;

            // at this point should insert to existing block
            self.dir_add_entry(child, &new_child_ref, "..")?;

            child.inode.set_links_count(2);
            let link_cnt = parent.inode.links_count() + 1;
            parent.inode.set_links_count(link_cnt);

            return Ok(EOK);
        }

        // Increment the link count of the child inode
        let link_cnt = child.inode.links_count() + 1;
        child.inode.set_links_count(link_cnt);

        Ok(EOK)
    }

    /// create a new inode and link it to the parent directory
    ///
    /// Params:
    /// parent: u32 - inode number of the parent directory
    /// name: &str - name of the new file
    /// mode: u16 - file mode
    ///
    /// Returns:
    pub fn create(&self, parent: u32, name: &str, inode_mode: u16) -> Result<Ext4InodeRef> {
        let mut parent_inode_ref = self.get_inode_ref(parent);

        // let mut child_inode_ref = self.create_inode(inode_mode)?;
        let init_child_ref = self.create_inode(inode_mode)?;

        self.write_back_inode_without_csum(&init_child_ref);
        // load new
        let mut child_inode_ref = self.get_inode_ref(init_child_ref.inode_num);

        self.link(&mut parent_inode_ref, &mut child_inode_ref, name)?;

        self.write_back_inode(&mut parent_inode_ref);
        self.write_back_inode(&mut child_inode_ref);

        Ok(child_inode_ref)
    }

    pub fn create_inode(&self, inode_mode: u16) -> Result<Ext4InodeRef> {
        let inode_file_type = InodeFileType::from_bits(inode_mode).unwrap();
        let is_dir = inode_file_type == InodeFileType::S_IFDIR;

        // allocate inode
        let inode_num = self.alloc_inode(is_dir)?;

        // initialize inode
        let mut inode = Ext4Inode::default();

        // set mode
        inode.set_mode(inode_mode | 0o777);

        // set extra size
        let inode_size = self.super_block.inode_size();
        let extra_size = self.super_block.extra_size();
        if inode_size > EXT4_GOOD_OLD_INODE_SIZE {
            let extra_size = extra_size;
            inode.set_i_extra_isize(extra_size);
        }

        // set extent
        inode.set_flags(EXT4_INODE_FLAG_EXTENTS as u32);
        inode.extent_tree_init();

        let inode_ref = Ext4InodeRef {
            inode_num: inode_num,
            inode: inode,
        };

        Ok(inode_ref)
    }

    /// Read data from a file at a given offset
    ///
    /// Parms:
    /// inode: u32 - inode number of the file
    /// offset: usize - offset from where to read
    /// buf: &mut [u8] - buffer to read the data into
    ///
    /// Returns:
    /// Result<usize> - number of bytes read
    pub fn read_at(&self, inode: u32, offset: usize, read_buf: &mut [u8]) -> Result<usize> {
        // read buf is empty, return 0
        let read_buf_len = read_buf.len();
        if read_buf_len == 0 {
            return Ok(0);
        }

        // get the inode reference
        let inode_ref = self.get_inode_ref(inode);

        // get the file size
        let file_size = inode_ref.inode.size();

        // if the offset is greater than the file size, return 0
        if offset >= file_size as usize {
            return Ok(0);
        }

        // adjust the read buffer size if the read buffer size is greater than the file size
        let size_to_read = min(read_buf_len, file_size as usize - offset);

        // calculate the start block
        let mut iblock = offset / BLOCK_SIZE;

        // unaligned size
        let unaligned_size = size_to_read % BLOCK_SIZE as usize;

        let mut cursor = 0;
        let mut total_bytes_read = 0;

        // adjust first block with unaligned size, remaining blocks are all full blocks
        if unaligned_size > 0 {
            // read the first block
            let adjust_read_size = min(BLOCK_SIZE - unaligned_size, size_to_read as usize);

            // get iblock physical block id
            let pblock_idx = self.get_pblock_idx(&inode_ref, iblock as u32)?;

            // read data
            let data = self
                .block_device
                .read_offset(pblock_idx as usize * BLOCK_SIZE);

            // copy data to read buffer
            read_buf[cursor..cursor + adjust_read_size].copy_from_slice(&data[..adjust_read_size]);

            // update cursor and total bytes read
            cursor += adjust_read_size;
            total_bytes_read += adjust_read_size;
            iblock += 1;
        }

        // Continue with full block reads
        while total_bytes_read < size_to_read {
            let read_length = core::cmp::min(BLOCK_SIZE, size_to_read - total_bytes_read);

            // get iblock physical block id
            let pblock_idx = self.get_pblock_idx(&inode_ref, iblock as u32)?;

            // read data
            let data = self
                .block_device
                .read_offset(pblock_idx as usize * BLOCK_SIZE);

            // copy data to read buffer
            read_buf[cursor..cursor + read_length].copy_from_slice(&data[..read_length]);

            // update cursor and total bytes read
            cursor += read_length;
            total_bytes_read += read_length;
            iblock += 1;
        }

        Ok(total_bytes_read)
    }

    /// Write data to a file at a given offset
    ///
    /// Params:
    /// inode: u32 - inode number of the file
    /// offset: usize - offset from where to write
    /// write_buf: &[u8] - buffer to write the data from
    ///
    /// Returns:
    /// Result<usize> - number of bytes written
    pub fn write_at(&self, inode: u32, offset: usize, write_buf: &[u8]) -> Result<usize> {
        // write buf is empty, return 0
        let write_buf_len = write_buf.len();
        if write_buf_len == 0 {
            return Ok(0);
        }

        // get the inode reference
        let mut inode_ref = self.get_inode_ref(inode);

        // Get the file size
        let file_size = inode_ref.inode.size();

        // Calculate the start and end block index
        let iblock_start = offset / BLOCK_SIZE;
        let iblock_last = (offset + write_buf_len) / BLOCK_SIZE;

        // start block index
        let mut iblk_idx = iblock_start;
        let ifile_blocks = (file_size + BLOCK_SIZE as u64 - 1) / BLOCK_SIZE as u64;

        // Calculate the unaligned size
        let unaligned = offset % BLOCK_SIZE;

        // Buffer to keep track of written bytes
        let mut written = 0;

        // Unaligned write
        if unaligned > 0 {
            let len = core::cmp::min(write_buf_len, BLOCK_SIZE - unaligned);
            // Get the physical block id, if the block is not present, append a new block
            let pblock_idx = if iblk_idx < ifile_blocks as usize {
                self.get_pblock_idx(&inode_ref, iblk_idx as u32)?
            } else {
                // physical block not exist, append a new block
                self.append_inode_pblk(&mut inode_ref)?
            };

            let mut block =
                Block::load(self.block_device.clone(), pblock_idx as usize * BLOCK_SIZE);

            block.write_offset(unaligned, &write_buf[..len], len);
            block.sync_blk_to_disk(self.block_device.clone());
            drop(block);

            trace!("Unaligned write at block: {}, size: {}", pblock_idx, len);

            written += len;
            iblk_idx += 1;
        }

        // Aligned write
        let mut fblock_start = 0;
        let mut fblock_count = 0;

        while written < write_buf_len {
            while iblk_idx < iblock_last {
                // Get the physical block id, if the block is not present, append a new block
                let pblock_idx = if iblk_idx < ifile_blocks as usize {
                    self.get_pblock_idx(&inode_ref, iblk_idx as u32)?
                } else {
                    // physical block not exist, append a new block
                    self.append_inode_pblk(&mut inode_ref)?
                };
                if fblock_start == 0 {
                    fblock_start = pblock_idx;
                }

                // Check if the block is contiguous
                if fblock_start + fblock_count != pblock_idx {
                    break;
                }

                fblock_count += 1;
                iblk_idx += 1;
            }

            // to do (write contiguos blocks at once)
            let len = core::cmp::min(fblock_count as usize * BLOCK_SIZE, write_buf_len - written);
            for i in 0..fblock_count {
                let block_offset = fblock_start as usize * BLOCK_SIZE + i as usize * BLOCK_SIZE;
                let mut block = Block::load(self.block_device.clone(), block_offset);
                block.write_offset(0, &write_buf[written..written + BLOCK_SIZE], BLOCK_SIZE);
                block.sync_blk_to_disk(self.block_device.clone());
                drop(block);
            }

            trace!("Aligned write at block: {}, size: {}", fblock_start, len);

            written += len as usize;
            fblock_start = 0;
            fblock_count = 0;
        }

        // Final unaligned write if any
        if written < write_buf_len {
            let len = write_buf_len - written;
            // Get the physical block id, if the block is not present, append a new block
            let pblock_idx = if iblk_idx < ifile_blocks as usize {
                self.get_pblock_idx(&inode_ref, iblk_idx as u32)?
            } else {
                // physical block not exist, append a new block
                self.append_inode_pblk(&mut inode_ref)?
            };

            let mut block =
                Block::load(self.block_device.clone(), pblock_idx as usize * BLOCK_SIZE);
            block.write_offset(0, &write_buf[written..], len);
            block.sync_blk_to_disk(self.block_device.clone());
            drop(block);

            written += len;
        }

        // Update file size if necessary
        if offset + write_buf_len > file_size as usize {
            inode_ref.inode.set_size((offset + write_buf_len) as u64);
        }

        Ok(written)
    }
}
