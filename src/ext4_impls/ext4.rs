use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Self> {
        // Load the superblock
        let block = Block::load(block_device.clone(), SUPERBLOCK_OFFSET);
        let super_block: Ext4Superblock = block.read_as();

        let ext4: Arc<Ext4> = Arc::new_cyclic(|weak_ref| Self {
            block_device,
            super_block: super_block,
            self_ref: weak_ref.clone(),
        });
        ext4
    }
}

