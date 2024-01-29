#![feature(error_in_core)]

extern crate alloc;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;

mod prelude;
mod ext4_defs;
mod ext4;
mod consts;
mod utils;
mod cstr;
mod ext4_error;

pub use ext4_error::*;
pub use prelude::*;
pub use ext4_defs::*;
pub use ext4::*;
pub use consts::*;
pub use utils::*;
pub use cstr::*;


// use lock::Mutex;
// use lock::MutexGuard;




#[derive(Debug)]
pub struct Disk {
}

impl BlockDevice for Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8>{
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

    fn write_offset(&self, offset: usize, data:&[u8]){
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


    let disk = Arc::new(Disk{});
    let ext4 = Ext4::open(disk);
    
}
