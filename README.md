# rust ext4 fs no_std

# write test

```shell
sh gen_write_img.sh
cargo test
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
    
    let mp = Ext4MountPoint::new("/");
    let path = "/dirtest1/dirtest2/../../dirtest1/dirtest2/dirtest3/dirtest4/dirtest5/../dirtest5/2.txt";
    let mut ext4_file = Ext4File::new(mp);
    
    ext4_generic_open::<Ext4TraitsImpl>( &mut ext4_file, path);
    ext4_file_read::<Ext4TraitsImpl>( &mut ext4_file);

```

