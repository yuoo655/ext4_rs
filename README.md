# a cross-platform rust ext4 crate support read/write/mkdir

# read/write example

rust version rustc 1.77.0-nightly   nightly-2023-12-28

```sh
git checkout dev
python3 gen_test_files.py
sh 1.sh
```


# how to use 

## impl BlockDevice Trait

```rust
pub struct Disk {}
impl BlockDevice for Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {}
    fn write_offset(&self, offset: usize, data: &[u8]) {}
}
```

## open ext4

```rust
let disk = Arc::new(Disk {});
let ext4 = Ext4::open(disk);
```

## read/write/mkdir

```rust

// read regular file
let path =
    "/test_files/1.txt";
let mut ext4_file = Ext4File::new();
ext4.ext4_open(&mut ext4_file, path, "r+", false);
log::info!("ext4_file inode {:?}", ext4_file.inode);
let data = ext4.ext4_file_read(&mut ext4_file);
log::info!("read data sample {:x?}", &data[0..10]);

// read link
let path =
"/test_files/linktest";
let mut ext4_file = Ext4File::new();
ext4.ext4_open(&mut ext4_file, path, "r+", false);
log::info!("ext4_file inode {:?}", ext4_file.inode);
let data = ext4.ext4_file_read(&mut ext4_file);
log::info!("read data sample {:x?}", &data[0..10]);

// dir
log::info!("----mkdir----");
for i in 0..10{
    let path = format!("dirtest{}", i);
    let path = path.as_str();
    ext4.ext4_dir_mk(&path);
}

// write test
// file
log::info!("----write file in dir----");
for i in 0..10{
    const write_size: usize = 4096 * 10;
    let path = format!("dirtest{}/write_{}.txt", i, i);
    let path = path.as_str();
    let mut ext4_file = Ext4File::new();
    ext4.ext4_open(&mut ext4_file, path, "w+", true);
    let write_data: [u8; write_size] = [0x41 + i as u8; write_size];
    ext4.ext4_file_write(&mut ext4_file, &write_data, write_size);
    // test
    ext4.ext4_open(&mut ext4_file, path, "r+", false);
    let data = ext4.ext4_file_read(&mut ext4_file);
    log::info!("read data sample {:x?}", &data[0..10]);
}
```
