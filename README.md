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



# how to use (old interface in dev branch)

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
let path = "/test_files/1.txt";
let mut ext4_file = Ext4File::new();
let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
assert!(r.is_ok(), "open file error {:?}", r.err());

let mut read_buf = vec![0u8; 0x20000000];
let mut read_cnt = 0;
let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, 0x20000000, &mut read_cnt);
assert!(r.is_ok(), "open file error {:?}", r.err());
```

### read link
```rust
log::info!("----read link file----");
let path = "/test_files/linktest";
let mut ext4_file = Ext4File::new();
let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
assert!(r.is_ok(), "open link error {:?}", r.err());

let mut read_buf = vec![0u8; 0x1000];
let mut read_cnt = 0;
let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, 0x1000, &mut read_cnt);
assert!(r.is_ok(), "read link error {:?}", r.err());
log::info!("read data sample {:x?}", &read_buf[0..10]);
```

### mkdir
```rust
for i in 0..10 {
    let path = format!("dirtest{}", i);
    let path = path.as_str();
    let r = ext4.ext4_dir_mk(&path);
    assert!(r.is_ok(), "dir make error {:?}", r.err());
}
```

### file write test
```rust
const WRITE_SIZE: usize = 0x400000;
let path = format!("dirtest{}/write_{}.txt", i, i);
let path = path.as_str();
let mut ext4_file = Ext4File::new();
let r = ext4.ext4_open(&mut ext4_file, path, "w+", true);
assert!(r.is_ok(), "open file error {:?}", r.err());

let write_data = vec![0x41 + i as u8; WRITE_SIZE];
ext4.ext4_file_write(&mut ext4_file, &write_data, WRITE_SIZE);

// test
let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
assert!(r.is_ok(), "open file error {:?}", r.err());

let mut read_buf = vec![0u8; WRITE_SIZE];
let mut read_cnt = 0;
let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, WRITE_SIZE, &mut read_cnt);
assert!(r.is_ok(), "open file error {:?}", r.err());
assert_eq!(write_data, read_buf);
```


### ls
```rust
let path = "test_files";
let mut ext4_file = Ext4File::new();
let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
assert!(r.is_ok(), "open link error {:?}", r.err());

let de = ext4.read_dir_entry(ext4_file.inode as _);
for i in de.iter() {
    log::info!("{:?}", i.get_name());
}
```

### file remove
```rust
let path = "test_files/file_to_remove";
let r = ext4.ext4_file_remove(&path);
```

### dir remove
```rust
let path = "dir_to_remove";
// remove dir from root inode 2
let r = ext4.ext4_dir_remove(2, &path);
```
