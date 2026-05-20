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

fn test_raw_block_device_write(block_device: Arc<dyn BlockDevice>, size_mb: usize) {
    let write_size = size_mb * 1024 * 1024;
    let mut buffer = vec![0x41u8; write_size];

    // Start from block 1000 to avoid overwriting important data
    let start_block = 1000;
    let start_offset = start_block * BLOCK_SIZE;

    log::info!("Starting raw BlockDevice write test: {} MB", size_mb);
    let start_time = std::time::Instant::now();

    // Write in BLOCK_SIZE chunks
    let mut written = 0;
    while written < write_size {
        let write_size = std::cmp::min(BLOCK_SIZE, write_size - written);
        let offset = start_offset + written;
        block_device.write_offset(offset, &buffer[written..written + write_size]);
        written += write_size;
    }

    let end_time = start_time.elapsed();
    let speed_mb_per_sec = (write_size as f64 / 1024.0 / 1024.0) / end_time.as_secs_f64();

    log::info!("Raw BlockDevice write speed: {:.2} MB/s", speed_mb_per_sec);
    log::info!("Total time: {:.2} seconds", end_time.as_secs_f64());
}

fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(LevelFilter::Trace);
    let disk = Arc::new(Disk {});
    let block_device = disk.clone(); // Clone before using
    let ext4 = Ext4::open(disk);

    // file read
    let path = "test_files/0.txt";
    // 1G
    const READ_SIZE: usize = (0x100000 * 1024);
    let mut read_buf = vec![0u8; READ_SIZE as usize];
    let child_inode = ext4.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
    let mut data = vec![0u8; READ_SIZE as usize];
    let read_data = ext4.read_at(child_inode, 0 as usize, &mut data);
    log::info!("read data  {:?}", &data[..10]);

    let path = "test_files/linktest";
    let mut read_buf = vec![0u8; READ_SIZE as usize];
    // 2 is root inode
    let child_inode = ext4.generic_open(path, &mut 2, false, 0, &mut 0).unwrap();
    let mut data = vec![0u8; READ_SIZE as usize];
    let read_data = ext4.read_at(child_inode, 0 as usize, &mut data);
    log::info!("read data  {:?}", &data[..10]);

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
    let inode_perm = (InodePerm::S_IREAD | InodePerm::S_IWRITE).bits();
    let inode_ref = ext4
        .create(ROOT_INODE, "4G.txt", inode_mode | inode_perm)
        .unwrap();
    log::info!("----write file----");
    const WRITE_SIZE: usize = (1024 * 1024 * 1024 * 4);
    let write_buf = vec![0x41 as u8; WRITE_SIZE];

    // Record start time
    let start_time = std::time::Instant::now();
    let r = ext4.write_at(inode_ref.inode_num, 0, &write_buf);
    let end_time = start_time.elapsed();

    // Calculate and display write speed
    let write_speed = (WRITE_SIZE as f64 / 1024.0 / 1024.0) / (end_time.as_secs_f64());
    log::info!("Write speed: {:.2} MB/s", write_speed);
    log::info!("Total time: {:.2} seconds", end_time.as_secs_f64());

    log::info!("----write done verifying----");
    const BLOCKS_PER_128MB: usize = 32768; // 128MB / 4KB = 32768 blocks
    let mut last_progress = 0;
    for i in 0..WRITE_SIZE / BLOCK_SIZE {
        let offset = (i * BLOCK_SIZE) as i64;
        let write_data = vec![0x41 as u8; BLOCK_SIZE];
        let read_data = ext4
            .ext4_file_read(inode_ref.inode_num as u64, BLOCK_SIZE as u32, offset)
            .unwrap();
        if read_data != write_data {
            log::info!("Data mismatch at block {:x}", i);
            panic!("Data mismatch at block {:x}", i);
        }

        // 每128MB打印一次进度
        let current_progress = i / BLOCKS_PER_128MB;
        if current_progress > last_progress {
            last_progress = current_progress;
            let progress_mb = current_progress * 128;
            log::info!(
                "Progress: {} MB / {} MB verified",
                progress_mb,
                WRITE_SIZE / (1024 * 1024)
            );
        }
    }
}
