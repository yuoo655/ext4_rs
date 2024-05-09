# a cross-platform rust ext4 crate support read/write/mkdir

## env
wsl2 ubuntu22.04

rust version nightly-2023-12-28

rustc 1.77.0-nightly   

## read/write example
```sh
git clone https://github.com/yuoo655/ext4_rs.git
git checkout dev
sh 1.sh
```


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

## read/write/mkdir

### read regular file
```rust
let path = "/test_files/1.txt";
let mut ext4_file = Ext4File::new();
let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
if let Err(e) = r {
    log::info!("open file error {:?}", e);
    panic!("open file error")
}else{
    let mut read_buf = vec![0u8; 0x20000000];
    let mut read_cnt = 0;
    let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, 0x20000000 , &mut read_cnt);
    if let Err(e) = r {
        log::info!("read file error {:?}", e);
        panic!("read file error")
    }
    log::info!("read data sample {:x?}", &read_buf[0..10]);
}
```

### read link
```rust
let path = "/test_files/linktest";
let mut ext4_file = Ext4File::new();
let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
if let Err(e) = r {
    log::info!("open file error {:?}", e);
    panic!("open file error")
}else{
    let mut read_buf = vec![0u8; 0x1000];
    let mut read_cnt = 0;
    let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, 0x1000 , &mut read_cnt);
    if let Err(e) = r {
        log::info!("read file error {:?}", e);
        panic!("read file error")
    }
    log::info!("read data sample {:x?}", &read_buf[0..10]);
}
```

### mkdir
```rust
log::info!("----mkdir----");
for i in 0..10{
    let path = format!("dirtest{}", i);
    let path = path.as_str();
    let r = ext4.ext4_dir_mk(&path);
    if let Err(e) = r {
        log::info!("dir make error {:?}", e);
        panic!("dir make error")
    }
}
```

### file write test
```rust
// file
log::info!("----write file in dir----");
for i in 0..10{
    const WRITE_SIZE: usize = 4096 * 10;
    let path = format!("dirtest{}/write_{}.txt", i, i);
    let path = path.as_str();
    let mut ext4_file = Ext4File::new();
    let r = ext4.ext4_open(&mut ext4_file, path, "w+", true);
    if let Err(e) = r {
        log::info!("open file error {:?}", e);
        panic!("open file error")
    }else{
        let write_data: [u8; WRITE_SIZE] = [0x41 + i as u8; WRITE_SIZE];
        ext4.ext4_file_write(&mut ext4_file, &write_data, WRITE_SIZE);
    }

    // test
    let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
    if let Err(e) = r {
        log::info!("open file error {:?}", e);
    }else {
        let mut read_buf = vec![0u8; 1024];
        let mut read_cnt = 0;
        let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, 10 , &mut read_cnt);
        if let Err(e) = r {
            log::info!("read file error {:?}", e);
            panic!("read file error")
        }
        log::info!("read data sample {:x?}", &read_buf[0..10]);
    }
}
```
