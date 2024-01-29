extern crate alloc;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;


mod defs;
mod ext4;

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


pub fn main() {
    let super_block = read_super_block::<Hal>();

    let idx = ext4_inode_alloc::<Hal>(1);
    let mut inode_data = read_inode::<Hal>(idx as u64, &super_block);

    ext4_inode_init(&mut inode_data, 0x1, false);
    ext4_extent_tree_init(&mut inode_data);

    //write back to device
    let block_offset = get_inode_block::<Hal>(idx as u64, &super_block);
    let mut write_back_data = [0u8; 0x9c];
    copy_inode_to_array(&inode_data, &mut write_back_data);
    Hal::write_block(block_offset, &write_back_data);

    let mut new_inode_data = read_inode::<Hal>(idx as u64, &super_block);

    println!("new_inode_data = {:#x?}", &new_inode_data);
    let mut child_inode_ref = Ext4InodeRef {
        inode: &mut new_inode_data,
        index: idx,
        dirty: false,
    };
    let mut root_inode = read_inode::<Hal>(2, &super_block);
    let path = "22222.txt";
    let name_len = 9;
    let mp = Ext4MountPoint::new("/");
    ext4_link::<Hal>(
        &mp,
        &root_inode,
        &mut child_inode_ref,
        path,
        name_len,
        false,
    );

    inode_data.links_count += 1;
    inode_data.i_extra_isize += 0x20;

    //write back to device
    let block_offset = get_inode_block::<Hal>(idx as u64, &super_block);
    let mut write_back_data = [0u8; 0x9c];
    copy_inode_to_array(&inode_data, &mut write_back_data);
    Hal::write_block(block_offset, &write_back_data);




    ext4_fs_set_inode_checksum::<Hal>(&mut root_inode, 2);
    ext4_fs_set_inode_checksum::<Hal>(&mut inode_data, idx);

    // // set extent
    let mut ext4_file = Ext4File::new(mp);
    ext4_file.inode = idx;

    let write_data: [u8; 4096 * 2] = [0x42 as u8; 4096 * 2];
    ext4_fwrite::<Hal>(&mut ext4_file, &write_data, 4096 * 2);

    let super_block = read_super_block::<Hal>();
    let mut new_data = read_inode::<Hal>(ext4_file.inode as u64, &super_block);
    new_data.blocks = 8 * 2;
    ext4_fs_set_inode_checksum::<Hal>(&mut root_inode, 2);
    ext4_fs_set_inode_checksum::<Hal>(&mut new_data, idx);

    let block_offset = get_inode_block::<Hal>(idx as u64, &super_block);
    let mut raw_data = Hal::read_block(block_offset);
    copy_inode_to_array(&new_data, &mut raw_data);
    Hal::write_block(block_offset, &raw_data);


    ext4_generic_open::<Hal>(&mut ext4_file, &path);

    let data = ext4_file_read_foo::<Hal>(&mut ext4_file);

    println!("data = {:x?}", &data[..10]);


}






