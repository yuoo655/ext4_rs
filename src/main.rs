#![feature(error_in_core)]

extern crate alloc;

mod prelude;
mod utils;

use prelude::*;
use utils::*;

mod ext4_defs;
mod ext4_impls;

mod fuse_interface;
mod simple_interface;

use fuse_interface::*;
use simple_interface::*;
use ext4_defs::*;

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

fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(LevelFilter::Info);
    let disk = Arc::new(Disk {});
    let ext4 = Ext4::open(disk);

    let entries = ext4.dir_get_entries(ROOT_INODE);
    for entry in entries {
        log::info!("entry: {:?}", entry.get_name());
    }
}
