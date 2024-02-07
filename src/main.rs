#![feature(error_in_core)]

extern crate alloc;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;

mod consts;
mod cstr;
mod ext4;
mod ext4_defs;
mod ext4_error;
mod prelude;
mod utils;

pub use consts::*;
pub use cstr::*;
pub use ext4::*;
pub use ext4_defs::*;
pub use ext4_error::*;
pub use prelude::*;
pub use utils::*;

// use lock::Mutex;
// use lock::MutexGuard;

#[derive(Debug)]
pub struct Disk {}

impl BlockDevice for Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        // println!("read_offset: {:x?}", offset);
        use std::fs::OpenOptions;
        use std::io::{Read, Seek};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();
        let mut buf = vec![0u8; BLOCK_SIZE as usize];
        let r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let r = file.read_exact(&mut buf);

        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        use std::fs::OpenOptions;
        use std::io::{Read, Seek, Write};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();

        let r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let r = file.write_all(&data);
    }
}

pub fn main() {
    let disk = Arc::new(Disk {});
    let ext4 = Ext4::open(disk);

    // read test
    let path =
        "/test_files/1.txt";
    let mut ext4_file = Ext4File::new();
    ext4.ext4_open(&mut ext4_file, path, "r+", false);
    println!("ext4_file inode {:?}", ext4_file.inode);
    let data = ext4.ext4_file_read(&mut ext4_file);
    println!("data sample {:x?}", &data[0..10]);


    // write test
    // file
    for i in 0..10{
        let path = format!("{}.txt", i);
        let path = path.as_str();
        let mut ext4_file = Ext4File::new();
        ext4.ext4_open(&mut ext4_file, path, "w+", true);
        // println!("ext4_file inode {:?}", ext4_file.inode);
        // let write_data: [u8; 8192] = [0x42 as u8; 8192];
        // ext4.ext4_file_write(&mut ext4_file, &write_data, 8192);
    }
    // // dir
    // for i in 0..10{
    //     let path = format!("dirtest{}", i);
    //     let path = path.as_str();
    //     let mut ext4_file = Ext4File::new();
    //     ext4.ext4_open(&mut ext4_file, path, "w", false);
    // }
}
