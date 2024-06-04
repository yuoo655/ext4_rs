use bitflags::bitflags;

pub const BLOCK_SIZE: usize = 4096;

pub type Ext4Lblk = u32;
pub type Ext4Fsblk = u64;

pub const EOK: usize = 0;

/// Inode
pub const EXT4_INODE_MODE_FILE: usize = 0x8000;
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;

/// Extent
pub const EXT_INIT_MAX_LEN: u16 = 32768;
pub const EXT_UNWRITTEN_MAX_LEN: u16 = 65535;


/// BLock group descriptor flags.
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;


/// SuperBlock
pub const SUPERBLOCK_OFFSET: usize = 1024;
pub const EXT4_SUPERBLOCK_OS_HURD: u32 = 1;