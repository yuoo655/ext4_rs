pub mod defs;
pub mod ext4;

pub use defs::*;
pub use ext4::*;

#[test]
fn test_write() {

    const TEST_WRITE_SIZE: usize = 0x20000;
    use crate::*;
    struct Hal {}
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

    let super_block = read_super_block::<Hal>();

    let idx = ext4_inode_alloc::<Hal>(1);
    let mut inode_data = read_inode::<Hal>(idx as u64, &super_block);

    ext4_inode_init(&mut inode_data, 0x1, false);
    ext4_extent_tree_init(&mut inode_data);

    //write back to device
    let block_offset = get_inode_block::<Hal>(idx as u64, &super_block);
    let mut write_back_data = [0u8; 0x80];
    copy_inode_to_array(&inode_data, &mut write_back_data);
    Hal::write_block(block_offset, &write_back_data);

    let mut new_inode_data = read_inode::<Hal>(idx as u64, &super_block);
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

    let write_data: [u8; TEST_WRITE_SIZE] = [0x42 as u8; TEST_WRITE_SIZE];
    ext4_fwrite::<Hal>(&mut ext4_file, &write_data, TEST_WRITE_SIZE as u64);

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
    // println!("data = {:x?}", &data[..10]);

    assert_eq!(data, write_data);
}
