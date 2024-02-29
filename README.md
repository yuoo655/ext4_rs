# a cross-platform rust ext4 crate

# read example

```sh
git checkout dev
python3 gen_test_files.py
sh 1.sh
```

# write example 

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

## read/write

```rust
// read test
let path =
    "/test_files/1.txt";
let mut ext4_file = Ext4File::new();
ext4.ext4_open(&mut ext4_file, path, "r+", false);
let data = ext4.ext4_file_read(&mut ext4_file);


// write test
// file
for i in 0..5{
    let path = format!("write_{}.txt", i);
    let path = path.as_str();
    let mut ext4_file = Ext4File::new();
    ext4.ext4_open(&mut ext4_file, path, "w+", true);
    let write_data: [u8; 8192] = [0x41 + i as u8; 8192];
    ext4.ext4_file_write(&mut ext4_file, &write_data, 8192);
    
    // test
    ext4.ext4_open(&mut ext4_file, path, "r+", false);
    let data = ext4.ext4_file_read(&mut ext4_file);
}

// dir
for i in 0..5{
    let path = format!("dirtest{}", i);
    let path = path.as_str();
    let mut ext4_file = Ext4File::new();
    ext4.ext4_open(&mut ext4_file, path, "w", false);
}
```
