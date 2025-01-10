# a cross-platform rust ext4 crate

[![Crates.io Version](https://img.shields.io/crates/v/ext4_rs)](https://crates.io/crates/ext4_rs)
[![Crates.io License](https://img.shields.io/crates/l/ext4_rs)](LICENSE)
[![docs.rs](https://img.shields.io/docsrs/ext4_rs)](https://docs.rs/ext4_rs)

## env
wsl2 ubuntu22.04

rust version nightly-2024-06-01

rustc 1.80.0-nightly (ada5e2c7b 2024-05-31)

mkfs.ext4 1.46.5 (30-Dec-2021) 

For small images, the newer mkfs.ext4 uses a 512-byte block size. Use **mkfs.ext4 -b 4096** to set a 4096-byte block size.

## run example
```sh
git clone https://github.com/yuoo655/ext4_rs.git
sh run.sh
```
## fuse example
```
git clone https://github.com/yuoo655/ext4libtest.git
cd ext4libtest
sh gen_img.sh
# cargo run /path/to/mountpoint
cargo run ./foo/
```
# features

| 操作         |支持情况| 
|--------------|------|
| mount        | ✅   |
| open         | ✅   |
| close        | ✅   |
| lsdir        | ✅   |
| mkdir        | ✅   |
| read_file    | ✅   |
| read_link    | ✅   |
| create_file  | ✅   |
| write_file   | ✅   |
| link         | ✅   |
| unlink       | ✅   |
| file_truncate| ✅   |
| file_remove  | ✅   |
| umount       | ✅   |
| dir_remove   | ✅   |



# how to use 

## impl BlockDevice Trait

```rust
#[derive(Debug)]
pub struct Disk {}

impl BlockDevice for Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        use std::fs::OpenOptions;
        use std::io::{Read, Seek};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();
        let mut buf = vec![0u8; BLOCK_SIZE as usize];
        let _r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let _r = file.read_exact(&mut buf);

        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        use std::fs::OpenOptions;
        use std::io::{Seek, Write};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("ex4.img")
            .unwrap();

        let _r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let _r = file.write_all(&data);
    }
}

```

## open ext4

```rust
let disk = Arc::new(Disk {});
let ext4 = Ext4::open(disk);
```

### read regular file
```rust
let path = "test_files/0.txt";
let mut read_buf = vec![0u8;  READ_SIZE as usize];
let child_inode = ext4.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
// 1G
let mut data = vec![0u8; 0x100000 * 1024 as usize];
let read_data = ext4.read_at(child_inode, 0 as usize, &mut data);
log::info!("read data  {:?}", &data[..10]);
```

### read link
```rust
let path = "test_files/linktest";
let mut read_buf = vec![0u8;  READ_SIZE as usize];
// 2 is root inode
let child_inode = ext4.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
let mut data = vec![0u8; 0x100000 * 1024 as usize];
let read_data = ext4.read_at(child_inode, 0 as usize, &mut data);
log::info!("read data  {:?}", &data[..10]);
```

### mkdir
```rust    
for i in 0..10 {
    let path = format!("dirtest{}", i);
    let path = path.as_str();
    let r = ext4.dir_mk(&path);
    assert!(r.is_ok(), "dir make error {:?}", r.err());
}
let path = "dir1/dir2/dir3/dir4/dir5/dir6";
let r = ext4.dir_mk(&path);
assert!(r.is_ok(), "dir make error {:?}", r.err());
```

### file write test
```rust
// file create/write
let inode_mode = InodeFileType::S_IFREG.bits();
let inode_ref = ext4.create(ROOT_INODE, "511M.txt", inode_mode).unwrap();

// test 511M  for 512M we need split the extent tree
const WRITE_SIZE: usize = (0x100000 * 511);
let write_buf = vec![0x41 as u8; WRITE_SIZE];
let r = ext4.write_at(inode_ref.inode_num, 0, &write_buf);
```


### ls
```rust
let entries = ext4.dir_get_entries(ROOT_INODE);
log::info!("dir ls root");
for entry in entries {
    log::info!("{:?}", entry.get_name());
}
```

### file remove
```rust
let path = "test_files/file_to_remove";
let r = ext4.file_remove(&path);
```

### dir remove
```rust
let path = "dir_to_remove";
let r = ext4.dir_remove(ROOT_INODE, &path);
```
