# rust ext4 crate no std

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
let path ="/dirtest1/dirtest2/../../dirtest1/dirtest2/dirtest3/dirtest4/dirtest5/../dirtest5/2.txt";
let mut ext4_file = Ext4File::new();
ext4.ext4_open(&mut ext4_file, path, "r+", false);
let data = ext4.ext4_file_read(&mut ext4_file);

let path ="1.txt"
ext4.ext4_open(&mut ext4_file, path, "wb", false);
let write_data = [0x41;4096]
ext4.ext4_file_write(&mut ext4_file, &write_data);
```
