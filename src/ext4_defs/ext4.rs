use crate::prelude::*;

use super::*;

pub struct Ext4 {
    pub block_device: Arc<dyn BlockDevice>,
    pub super_block: Ext4Superblock,
}
