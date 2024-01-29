# rust ext4 fs no_std

# run write example
```shell
sh gen_write_img.sh
cargo run
```

# ext4trait

impl Ext4Traits

```rust
impl Ext4Traits for Ext4TraitsImpl{

    fn read_block(offset: u64) ->Vec<u8> {
        use std::fs::OpenOptions;
        use std::io::{Read, Seek};
        let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .open("ex4.img")
        .unwrap();
        let mut buf = vec![0u8; BLOCK_SIZE as usize];
        let r = file.seek(std::io::SeekFrom::Start(offset));
        let r = file.read_exact(&mut buf);
        buf
    }
    fn write_block(offset: u64, buf: &[u8]) {]
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
```
