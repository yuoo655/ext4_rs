use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
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
}
