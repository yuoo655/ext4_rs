#![feature(error_in_core)]

extern crate alloc;

use alloc::vec;

mod consts;
mod ext4_structs;
mod ext4_error;
mod prelude;
mod utils;
mod ext4_interface;
mod ext4_impl;

pub use consts::*;
pub use ext4_interface::*;
pub use ext4_structs::*;
pub use ext4_error::*;
use prelude::*;
pub use utils::*;



use log::{error, Level, LevelFilter, Metadata, Record};

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

        debug!("write offset {:x?}", offset);
        let _r = file.seek(std::io::SeekFrom::Start(offset as u64));
        let _r = file.write_all(&data);
    }
}

pub fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(LevelFilter::Trace);
    let disk = Arc::new(Disk {});
    let ext4 = Ext4::open(disk);

    let path = "/test_files/3.txt";

    // checksum fail (need linux crypto subsystem) 
    let r = ext4.ext4_file_remove(path);
}
