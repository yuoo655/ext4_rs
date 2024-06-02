use bitflags::bitflags;

pub const BASE_OFFSET: usize = 1024;
pub const BLOCK_SIZE: usize = 4096;

pub const EOK: usize = 0;
pub type Ext4Lblk = u32;
pub type Ext4Fsblk = u64;

pub const EXT4_INODE_FLAG_EXTENTS: usize = 0x00080000; /* Inode uses extents */
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;
pub const EXT4_CRC32_INIT: u32 = 0xFFFFFFFF;
pub const EXT4_EXTENT_MAGIC: u16 = 0xF30A;
pub const EXT_INIT_MAX_LEN: u16 = 32768;
pub const EXT_UNWRITTEN_MAX_LEN: u16 = 65535;

pub const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;

pub const EXT4_INODE_MODE_FIFO: usize = 0x1000;
pub const EXT4_INODE_MODE_CHARDEV: usize = 0x2000;
pub const EXT4_INODE_MODE_DIRECTORY: usize = 0x4000;
pub const EXT4_INODE_MODE_BLOCKDEV: usize = 0x6000;
pub const EXT4_INODE_MODE_FILE: usize = 0x8000;
pub const EXT4_INODE_MODE_SOFTLINK: usize = 0xA000;
pub const EXT4_INODE_MODE_SOCKET: usize = 0xC000;
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;

pub const EXT4_INODE_BLOCK_SIZE:usize = 512;
pub const EXT_MAX_BLOCKS: Ext4Lblk = core::u32::MAX;

pub const EXT4_SUPERBLOCK_OS_HURD: u32 = 1;

/// Maximum bytes in a path
pub const PATH_MAX: usize = 4096;

/// Maximum bytes in a file name
pub const NAME_MAX: usize = 255;

/// The upper limit for resolving symbolic links
pub const SYMLINKS_MAX: usize = 40;

#[derive(Debug, PartialEq)]
pub enum LibcOpenFlags {
    OACCMODE,
    ORDONLY,
    OWRONLY,
    ORDWR,
    OCREAT,
    OEXCL,
    ONOCTTY,
    OTRUNC,
    OAPPEND,
    ONONBLOCK,
    OSYNC,
    OASYNC,
    OLARGEFILE,
    ODIRECTORY,
    ONOFOLLOW,
    OCLOEXEC,
    ODIRECT,
    ONOATIME,
    OPATH,
    ODSYNC,
}

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

pub const EPERM: usize = 1; /* Operation not permitted */
pub const ENOENT: usize = 2; /* No such file or directory */
pub const ESRCH: usize = 3; /* No such process */
pub const EINTR: usize = 4; /* Interrupted system call */
pub const EIO: usize = 5; /* I/O error */
pub const ENXIO: usize = 6; /* No such device or address */
pub const E2BIG: usize = 7; /* Argument list too long */
pub const ENOEXEC: usize = 8; /* Exec format error */
pub const EBADF: usize = 9; /* Bad file number */
pub const ECHILD: usize = 10; /* No child processes */
pub const EAGAIN: usize = 11; /* Try again */
pub const ENOMEM: usize = 12; /* Out of memory */
pub const EACCES: usize = 13; /* Permission denied */
pub const EFAULT: usize = 14; /* Bad address */
pub const ENOTBLK: usize = 15; /* Block device required */
pub const EBUSY: usize = 16; /* Device or resource busy */
pub const EEXIST: usize = 17; /* File exists */
pub const EXDEV: usize = 18; /* Cross-device link */
pub const ENODEV: usize = 19; /* No such device */
pub const ENOTDIR: usize = 20; /* Not a directory */
pub const EISDIR: usize = 21; /* Is a directory */
pub const EINVAL: usize = 22; /* Invalid argument */
pub const ENFILE: usize = 23; /* File table overflow */
pub const EMFILE: usize = 24; /* Too many open files */
pub const ENOTTY: usize = 25; /* Not a typewriter */
pub const ETXTBSY: usize = 26; /* Text file busy */
pub const EFBIG: usize = 27; /* File too large */
pub const ENOSPC: usize = 28; /* No space left on device */
pub const ESPIPE: usize = 29; /* Illegal seek */
pub const EROFS: usize = 30; /* Read-only file system */
pub const EMLINK: usize = 31; /* Too many links */
pub const EPIPE: usize = 32; /* Broken pipe */
pub const EDOM: usize = 33; /* Math argument out of domain of func */
pub const ERANGE: usize = 34; /* Math result not representable */

bitflags! {
    pub struct OpenFlag: u32 {
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

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct InodeMode: u16 {
        const S_IFSOCK = 0xC000;
        const S_IFLNK = 0xA000;
        const S_IFREG = 0x8000;
        const S_IFBLK = 0x6000;
        const S_IFDIR = 0x4000;
        const S_IFCHR = 0x2000;
        const S_IFIFO = 0x1000;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct FileMode: u16 {
        const S_IFMT   = 0xF000; // 文件类型的位域
        const S_IFSOCK = 0xC000; // 套接字
        const S_IFLNK  = 0xA000; // 符号链接
        const S_IFREG  = 0x8000; // 普通文件
        const S_IFBLK  = 0x6000; // 块设备
        const S_IFDIR  = 0x4000; // 目录
        const S_IFCHR  = 0x2000; // 字符设备
        const S_IFIFO  = 0x1000; // FIFO
        const S_ISUID  = 0x0800; // 设置用户ID位
        const S_ISGID  = 0x0400; // 设置组ID位
        const S_ISVTX  = 0x0200; // 粘滞位
        const S_IRWXU  = 0x01E0; // 用户权限的位域
        const S_IRUSR  = 0x0100; // 用户可读
        const S_IWUSR  = 0x0080; // 用户可写
        const S_IXUSR  = 0x0040; // 用户可执行
        const S_IRWXG  = 0x001C; // 组权限的位域
        const S_IRGRP  = 0x0010; // 组可读
        const S_IWGRP  = 0x0008; // 组可写
        const S_IXGRP  = 0x0004; // 组可执行
        const S_IRWXO  = 0x0007; // 其他用户权限的位域
        const S_IROTH  = 0x0004; // 其他用户可读
        const S_IWOTH  = 0x0002; // 其他用户可写
        const S_IXOTH  = 0x0001; // 其他用户可执行
    }
}
