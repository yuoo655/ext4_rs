pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;
pub const EXT4_CRC32_INIT: u32 = 0xFFFFFFFF;
/// Maximum bytes in a path
pub const PATH_MAX: usize = 4096;

/// Maximum bytes in a file name
pub const NAME_MAX: usize = 255;

/// The upper limit for resolving symbolic links
pub const SYMLINKS_MAX: usize = 40;
