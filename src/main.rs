#![feature(error_in_core)]

extern crate alloc;

use alloc::vec;

mod consts;
mod ext4_error;
mod ext4_impl;
mod ext4_interface;
mod ext4_structs;
mod prelude;
mod utils;

pub use consts::*;
pub use ext4_error::*;
pub use ext4_interface::*;
pub use ext4_structs::*;
use prelude::*;
pub use utils::*;

use log::{Level, LevelFilter, Metadata, Record};

macro_rules! with_color {
    ($color_code:expr, $($arg:tt)*) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[m", $color_code as u8, format_args!($($arg)*))
    }};
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        let level = record.level();
        let args_color = match level {
            Level::Error => ColorCode::Red,
            Level::Warn => ColorCode::Yellow,
            Level::Info => ColorCode::Green,
            Level::Debug => ColorCode::Cyan,
            Level::Trace => ColorCode::BrightBlack,
        };

        if self.enabled(record.metadata()) {
            println!(
                "{} - {}",
                record.level(),
                with_color!(args_color, "{}", record.args())
            );
        }
    }

    fn flush(&self) {}
}

#[repr(u8)]
enum ColorCode {
    Red = 31,
    Green = 32,
    Yellow = 33,
    Cyan = 36,
    BrightBlack = 90,
}

#[derive(Debug)]
pub struct Disk {}

impl BlockDevice for Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        // log::info!("read_offset: {:x?}", offset);
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

pub fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(LevelFilter::Info);
    let disk = Arc::new(Disk {});
    let ext4 = Ext4::open(disk);

    // read regular file
    log::info!("----read regular file----");
    let path = "/test_files/1.txt";
    let mut ext4_file = Ext4File::new();
    let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
    assert!(r.is_ok(), "open file error {:?}", r.err());

    let mut read_buf = vec![0u8; 0x20000000];
    let mut read_cnt = 0;
    let r = ext4.ext4_file_read(&mut ext4_file, &mut read_buf, 0x20000000, &mut read_cnt);
    assert!(r.is_ok(), "open file error {:?}", r.err());

    log::info!("read data sample {:x?}", &read_buf[0..10]);

    // read link
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

    // dir
    log::info!("----mkdir----");
    for i in 0..10 {
        let path = format!("dirtest{}", i);
        let path = path.as_str();
        let r = ext4.ext4_dir_mk(&path);
        assert!(r.is_ok(), "dir make error {:?}", r.err());
    }

    // write test
    // file
    log::info!("----write file in dir----");
    for i in 0..10 {
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
    }

    // ls
    log::info!("----ls----");
    let path = "test_files";
    let mut ext4_file = Ext4File::new();
    let r = ext4.ext4_open(&mut ext4_file, path, "r+", false);
    assert!(r.is_ok(), "open link error {:?}", r.err());

    let de = ext4.read_dir_entry(ext4_file.inode as _);
    for i in de.iter() {
        log::info!("{:?}", i.get_name());
    }

    //file remove
    log::info!("----file remove----");
    let path = "test_files/file_to_remove";
    let r = ext4.ext4_file_remove(&path);

    //dir remove
    log::info!("----dir remove----");
    let path = "dir_to_remove";
    let r = ext4.ext4_dir_remove(2, &path);
}
