extern crate alloc;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;



use crate::prelude::*;
use crate::defs::*;

pub (crate) const BASE_OFFSET: usize = 0x1000;
pub (crate) const BLOCK_SIZE: usize = 4096;


pub trait BlockDevice: Send + Sync + Any + Debug {
    fn read_offset(&self, offset: usize) -> Vec<u8>;
    fn write_offset(&self);
}

impl dyn BlockDevice {
    pub fn downcast_ref<T: BlockDevice>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }
}



#[derive(Debug)]
pub struct Ext4 {
    block_device: Arc<dyn BlockDevice>,
    super_block: Ext4Superblock,
    block_groups: Vec<Ext4BlockGroup>,
    inodes_per_group: u32,
    blocks_per_group: u32,
    inode_size: usize,
    self_ref: Weak<Self>,
}

impl Ext4 {
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Self>{
        // Load the superblock
        // TODO: if the main superblock is corrupted, should we load the backup?
        let super_block = {
            let raw_data = block_device.read_offset(BASE_OFFSET);
            Ext4Superblock::try_from(raw_data).unwrap()
        };

        let inodes_per_group = super_block.inodes_per_group();
        let blocks_per_group = super_block.blocks_per_group();
        let inode_size = super_block.inode_size();

        // Load the block groups information
        let load_block_groups = |fs: Weak<Ext4>, block_device: &dyn BlockDevice|-> Result<Vec<Ext4BlockGroup>> {
            let block_groups_count = super_block.block_groups_count() as usize;
            let mut block_groups = Vec::with_capacity(block_groups_count);
            for idx in 0..block_groups_count {
                let block_group = Ext4BlockGroup::load(
                    idx,
                    block_device,
                    &super_block,
                    fs.clone(),
                )?;
                block_groups.push(block_group);
            }
            Ok(block_groups)
        };


        let ext4 = Arc::new_cyclic(|weak_ref| Self {
            super_block: super_block,
            inodes_per_group: inodes_per_group,
            blocks_per_group: blocks_per_group,
            inode_size: inode_size as usize,
            block_groups: load_block_groups(
                weak_ref.clone(),
                block_device.as_ref(),
            )
            .unwrap(),
            block_device,
            self_ref: weak_ref.clone(),
        });

        ext4
    
    }
}


pub fn ext4_fopen(file: &Ext4FileNew, path: &str, flags: &str) {}

pub fn ext4_generic_open_clean(file: &Ext4FileNew, path: &str, flags: &str) {}



