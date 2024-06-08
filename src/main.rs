#![feature(error_in_core)]
#![allow(unused)]

extern crate alloc;

mod prelude;
mod utils;

use prelude::*;
use utils::*;

mod ext4_defs;
mod ext4_impls;

mod fuse_interface;
mod simple_interface;

use ext4_defs::*;
use fuse_interface::*;
use simple_interface::*;

use log::{Level, LevelFilter, Metadata, Record};

macro_rules! with_color {
    ($color_code:expr, $($arg:tt)*) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[m", $color_code as u8, format_args!($($arg)*))
    }};
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
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

fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(LevelFilter::Trace);
    let disk = Arc::new(Disk {});
    let ext4 = Ext4::open(disk);

    // dir make
    log::info!("----mkdir----");
    for i in 0..10 {
        let path = format!("dirtest{}", i);
        let path = path.as_str();
        log::info!("mkdir making {:?}", path);
        let r = ext4.dir_mk(&path);
        assert!(r.is_ok(), "dir make error {:?}", r.err());
    }
    let path = "dir1/dir2/dir3/dir4/dir5/dir6";
    log::info!("mkdir making {:?}", path);
    let r = ext4.dir_mk(&path);
    assert!(r.is_ok(), "dir make error {:?}", r.err());

    // dir ls
    let entries = ext4.dir_get_entries(ROOT_INODE);
    log::info!("dir ls root");
    for entry in entries {
        log::info!("{:?}", entry.get_name());
    }

    // file remove
    let path = "test_files/file_to_remove";
    let r = ext4.file_remove(&path);

    // dir remove
    let path = "dir_to_remove";
    let r = ext4.dir_remove(ROOT_INODE, &path);

    // file create/write
    log::info!("----create file----");
    let inode_mode = InodeFileType::S_IFREG.bits();
    let inode_ref = ext4.create(ROOT_INODE, "511M.txt", inode_mode).unwrap();
    log::info!("----write file----");
    // test 511M  for 512M we need split the extent tree
    const WRITE_SIZE: usize = (0x100000 * 511);
    let write_buf = vec![0x41 as u8; WRITE_SIZE];
    let r = ext4.write_at(inode_ref.inode_num, 0, &write_buf);
}
