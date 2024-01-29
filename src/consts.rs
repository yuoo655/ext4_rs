use bitflags::bitflags;

pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;
pub const EXT4_CRC32_INIT: u32 = 0xFFFFFFFF;
/// Maximum bytes in a path
pub const PATH_MAX: usize = 4096;

/// Maximum bytes in a file name
pub const NAME_MAX: usize = 255;

/// The upper limit for resolving symbolic links
pub const SYMLINKS_MAX: usize = 40;


pub const O_ACCMODE: u32 = 0o0003;
pub const O_RDONLY: u32 = 0o00;
pub const O_WRONLY: u32 = 0o01;
pub const O_RDWR: u32 = 0o02;
pub const O_CREAT: u32 = 0o0100;
pub const O_EXCL: u32 = 0o0200;
pub const O_NOCTTY: u32 = 0o0400;
pub const O_TRUNC: u32 = 0o01000;
pub const O_APPEND: u32 = 0o02000;
pub const O_NONBLOCK: u32 = 0o04000;
pub const O_SYNC: u32 = 0o4010000;
pub const O_ASYNC: u32 = 0o020000;
pub const O_LARGEFILE: u32 = 0o0100000;
pub const O_DIRECTORY: u32 = 0o0200000;
pub const O_NOFOLLOW: u32 = 0o0400000;
pub const O_CLOEXEC: u32 = 0o2000000;
pub const O_DIRECT: u32 = 0o040000;
pub const O_NOATIME: u32 = 0o1000000;
pub const O_PATH: u32 = 0o10000000;
pub const O_DSYNC: u32 = 0o010000;




bitflags! {
    pub struct OFlag: u32 {
        // 以下是open/fcntl的一些常量，和C语言的值相同
        const O_ACCMODE = 0o0003;
        const O_RDONLY = 0o00;
        const O_WRONLY = 0o01;
        const O_RDWR = 0o02;
        const O_CREAT = 0o0100;
        const O_EXCL = 0o0200;
        const O_NOCTTY = 0o0400;
        const O_TRUNC = 0o01000;
        const O_APPEND = 0o02000;
        const O_NONBLOCK = 0o04000;
        const O_NDELAY = Self::O_NONBLOCK.bits();
        const O_SYNC = 0o4010000;
        const O_FSYNC = Self::O_SYNC.bits();
        const O_ASYNC = 0o020000;
        const O_LARGEFILE = 0o0100000;
        const O_DIRECTORY = 0o0200000;
        const O_NOFOLLOW = 0o0400000;
        const O_CLOEXEC = 0o2000000;
        const O_DIRECT = 0o040000;
        const O_NOATIME = 0o1000000;
        const O_PATH = 0o10000000;
        const O_DSYNC = 0o010000;
        const O_TMPFILE = 0o20000000 | Self::O_DIRECTORY.bits();
    }
}