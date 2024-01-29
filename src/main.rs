extern crate alloc;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;

mod prelude;
mod defs;
mod ext4;

use crate::prelude::*;
use crate::defs::*;
use crate::ext4::*;

pub struct Hal {}

impl Ext4Traits for Hal {
    fn read_block(offset: u64) -> Vec<u8> {
        // println!("read offset {:x?}", offset);
        use std::fs::OpenOptions;
        use std::io::{Read, Seek};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();

        // println!("read offset ={:x?}\n", offset);
        let mut buf = vec![0u8; BLOCK_SIZE as usize];
        let r = file.seek(std::io::SeekFrom::Start(offset));
        let r = file.read_exact(&mut buf);

        buf
    }

    fn write_block(offset: u64, buf: &[u8]) {
        // println!("write offset {:x?}", offset);
        use std::fs::OpenOptions;
        use std::io::{Read, Seek, Write};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();

        let r = file.seek(std::io::SeekFrom::Start(offset));
        let r = file.write_all(&buf);
    }
}

// use lock::Mutex;
// use lock::MutexGuard;

pub trait BlockDevice: Send + Sync + Any + Debug {
    fn read_offset(&self, offset: usize) -> Vec<u8>;
    fn write_offset(&self);
}
#[derive(Debug)]
pub struct Ext4 {
    block_device: Arc<dyn BlockDevice>,
    super_block: Ext4Superblock,
    block_groups: Vec<Ext4BlockGroup>,
    inodes_per_group: u32,
    blocks_per_group: u32,
    inode_size: usize,
    block_size: usize,
    self_ref: Weak<Self>,
}

impl Ext4 {
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) {
        // Load the superblock
        // TODO: if the main superblock is corrupted, should we load the backup?
        let super_block = {
            let raw_data = block_device.read_offset(BASE_OFFSET)?;
            Ext4Superblock::try_from(raw_data)?
        };
        assert!(super_block.block_size() == BLOCK_SIZE);
    }
}

impl dyn BlockDevice {
    pub fn downcast_ref<T: BlockDevice>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }
}

pub fn ext4_fopen(file: &Ext4FileNew, path: &str, flags: &str) {}

pub fn ext4_generic_open_clean(file: &Ext4FileNew, path: &str, flags: &str) {}

pub fn main() {
    let f = Ext4FileNew::new();
    let path = "test.txt";
    let r = ext4_fopen(&f, path, "wb");
}
