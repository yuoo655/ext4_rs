use bitflags::bitflags;
use core::mem::size_of;
use std::marker::PhantomData;


// 定义超级块结构体，参考 https://www.nongnu.org/ext2-doc/ext2.html#SUPERBLOCK
#[repr(C)]
#[derive(Debug)]
pub struct Ext4SuperBlock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub r_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_frag_size: u32,
    pub blocks_per_group: u32,
    pub frags_per_group: u32,
    pub inodes_per_group: u32,
    pub mtime: u32,
    pub wtime: u32,
    pub mnt_count: u16,
    pub max_mnt_count: u16,
    pub magic: u16,
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
    // 以下字段仅适用于ext2 rev 1或更高版本
    pub first_ino: u32,
    pub inode_size: u16,
    pub block_group_nr: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub uuid: [u8; 16],
    pub volume_name: [u8; 16],
    pub last_mounted: [u8; 64],
    pub algo_bitmap: u32,
    // 以下字段仅适用于ext3
    pub prealloc_blocks: u8,
    pub prealloc_dir_blocks: u8,
    pub reserved_gdt_blocks: u16,
    pub journal_uuid: [u8; 16],
    pub journal_inum: u32,
    pub journal_dev: u32,
    pub last_orphan: u32,
    pub hash_seed: [u32; 4],
    pub def_hash_version: u8,
    pub journal_backup_type: u8,
	pub desc_size: u16,
    pub default_mount_options: u32,
    pub first_meta_bg: u32,
    pub mkfs_time: u32,
    pub journal_blocks: [u32; 17],
    // 以下字段仅适用于ext4
    pub blocks_count_hi: u32,
    pub r_blocks_count_hi: u32,
    pub free_blocks_count_hi: u32,
    pub min_extra_isize: u16,
    pub want_extra_isize: u16,
    pub flags: u32,
    pub raid_stride: u16,
    pub mmp_interval: u16,
    pub mmp_block: u64,
    pub raid_stripe_width: u32,
    pub log_groups_per_flex: u8,
    pub checksum_type: u8,
    pub reserved_pad: u16,
    pub kbytes_written: u64,
    pub snapshot_inum: u32,
    pub snapshot_id: u32,
    pub snapshot_r_blocks_count: u64,
    pub snapshot_list: u32,
    pub error_count: u32,
    pub first_error_time: u32,
    pub first_error_ino: u32,
    pub first_error_block: u64,
    pub first_error_func: [u8; 32],
    pub first_error_line: u32,
    pub last_error_time: u32,
    pub last_error_ino: u32,
    pub last_error_line: u32,
    pub last_error_block: u64,
    pub last_error_func: [u8; 32],
    pub mount_opts: [u8; 64],
    pub usr_quota_inum: u32,
    pub grp_quota_inum: u32,
    pub overhead_blocks: u32,
    pub backup_bgs: [u32; 2],
    pub encrypt_algos: [u8; 4],
    pub encrypt_pw_salt: [u8; 16],
    pub lpf_ino: u32,
    pub prj_quota_inum: u32,
    pub checksum_seed: u32,
    pub padding2: [u8; 100],
    pub checksum: u32,
}

// 定义inode结构体，参考 https://www.nongnu.org/ext2-doc/ext2.html#INODES
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Inode {
    pub mode: u16,
    pub uid: u16,
    pub size: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks: u32,
    pub flags: u32,
    pub osd1: u32,
    pub block: [u32; 15],
    pub generation: u32,
    pub file_acl: u32,
    pub size_hi: u32,
    pub faddr: u32,     /* Obsoleted fragment address */
    pub osd2: Linux2, // 操作系统相关的字段2

    pub i_extra_isize: u16,
    pub i_checksum_hi: u16, // crc32c(uuid+inum+inode) BE
    pub i_ctime_extra: u32, // 额外的修改时间（nsec << 2 | epoch）
    pub i_mtime_extra: u32, // 额外的文件修改时间（nsec << 2 | epoch）
    pub i_atime_extra: u32, // 额外的访问时间（nsec << 2 | epoch）
    pub i_crtime: u32, // 文件创建时间
    pub i_crtime_extra: u32, // 额外的文件创建时间（nsec << 2 | epoch）
    pub i_version_hi: u32, // 64位版本的高32位
}

#[derive(Debug)]
pub struct Ext4InodeRef<'a> {
    pub inode: &'a Ext4Inode, // 
    pub index: u32, // inode的索引号
    pub dirty: bool, // inode是否被修改过
}

#[derive(Debug)]
pub struct Ext4InodeRefNew<'a, A:Ext4Traits> {
    pub block: Ext4BlockNew<'a, A>, // ext4块
    pub inode: Ext4Inode, // ext4 inode的原始指针
    pub index: u32, // inode的索引号
    pub dirty: bool, // inode是否被修改过
}

impl <'a, A:Ext4Traits> Ext4InodeRefNew<'a, A> {

    // pub fn new(inode: u64, super_block: &Ext4SuperBlock, inode_table_blk_num: u64) -> Self{


    //     let inodes_per_group = super_block.inodes_per_group;
    //     let inode_size = super_block.inode_size as u64;
    //     let group = (inode - 1) / inodes_per_group as u64;
    //     let index = (inode - 1) % inodes_per_group as u64;
    //     let offset = inode_table_blk_num * BLOCK_SIZE + index * inode_size;
    
    //     let data = A::read_block(offset);

    //     let mut buf = [0u8; size_of::<Ext4Inode>()];
    //     buf.copy_from_slice(&data[..size_of::<Ext4Inode>()]);
    //     let inode_data = unsafe { core::ptr::read(buf.as_ptr() as *const Ext4Inode) };

    //     let mut new_data = A::read_block(offset);

    //     // let block =  Ext4BlockNew::<A>{
    //     //     disk_block_id: offset / BLOCK_SIZE,
    //     //     block_data: &mut new_data,
    //     //     dirty: false,
    //     //     phantom: PhantomData
    //     // };

    //     let mut inode_ref = Ext4InodeRefNew::<A>{
    //         block: Ext4BlockNew::<A>{
    //             disk_block_id: offset / BLOCK_SIZE,
    //             block_data: &mut new_data,
    //             dirty: false,
    //             phantom: PhantomData
    //         },
    //         inode: inode_data,
    //         index: inode as u32,
    //         dirty: false,

    //     };


    //     inode_ref


    //     // let inode_data = &inode_data;

    // }
    
}


/**@brief   Mount point descriptor.*/
pub struct Ext4MountPoint {
    /**@brief   Mount done flag.*/
    pub mounted: bool,
    /**@brief   Mount point name (@ref ext4_mount)*/
    pub mount_name: [char; 33],

    pub mount_name_string: String,
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct GroupDesc {
    /// Lower 32-bits of location of block bitmap.
    pub bg_block_bitmap_lo: u32,

    /// Lower 32-bits of location of inode bitmap.
    pub bg_inode_bitmap_lo: u32,

    /// Lower 32-bits of location of inode table.
    pub bg_inode_table_lo: u32,

    /// Lower 16-bits of free block count.
    pub bg_free_blocks_count_lo: u16,

    /// Lower 16-bits of free inode count.
    pub bg_free_inodes_count_lo: u16,

    /// Lower 16-bits of directory count.
    pub bg_used_dirs_count_lo: u16,

    /// Block group flags
    pub bg_flags: GroupFlags,

    /// Lower 32-bits of location of snapshot exclusion bitmap.
    pub bg_exclude_bitmap_lo: u32,

    /// Lower 16-bits of the block bitmap checksum.
    pub bg_block_bitmap_csum_lo: u16,

    /// Lower 16-bits of the inode bitmap checksum.
    pub bg_inode_bitmap_csum_lo: u16,

    /// Lower 16-bits of unused inode count.
    /// If set, we needn’t scan past the (sb.s_inodes_per_group - gdt.bg_itable_unused) th
    /// entry in the inode table for this group.
    pub bg_itable_unused_lo: u16,

    /// Group descriptor checksum;
    /// crc16(sb_uuid+group_num+bg_desc) if the RO_COMPAT_GDT_CSUM feature is set,
    /// or crc32c(sb_uuid+group_num+bg_desc) & 0xFFFF if the RO_COMPAT_METADATA_CSUM feature is set.
    /// The bg_checksum field in bg_desc is skipped when calculating crc16 checksum,
    /// and set to zero if crc32c checksum is used.
    pub bg_checksum: u16,

    /// Upper 32-bits of location of block bitmap.
    pub bg_block_bitmap_hi: u32,

    /// Upper 32-bits of location of inodes bitmap.
    pub bg_inode_bitmap_hi: u32,

    /// Upper 32-bits of location of inodes table.
    pub bg_inode_table_hi: u32,

    /// Upper 16-bits of free block count.
    pub bg_free_blocks_count_hi: u16,

    /// Upper 16-bits of free inode count.
    pub bg_free_inodes_count_hi: u16,

    /// Upper 16-bits of directory count.
    pub bg_used_dirs_count_hi: u16,

    /// Upper 16-bits of unused inode count.
    pub bg_itable_unused_hi: u16,

    /// Upper 32-bits of location of snapshot exclusion bitmap.
    pub bg_exclude_bitmap_hi: u32,

    /// Upper 16-bits of the block bitmap checksum.
    pub bg_block_bitmap_csum_hi: u16,

    /// Upper 16-bits of the inode bitmap checksum.
    pub bg_inode_bitmap_csum_hi: u16,

    /// Padding to 64 bytes.
    pub bg_reserved: u32,
}

// 定义目录项结构体，参考 https://www.nongnu.org/ext2-doc/ext2.html#DIRECTORY-ENTRIES
#[repr(C)]
#[derive(Debug)]
pub struct Ext4DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    pub name: [u8; 255],
}


#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentHeader {
    /// Magic number, 0xF30A.
    pub eh_magic: u16,

    /// Number of valid entries following the header.
    pub eh_entries: u16,

    /// Maximum number of entries that could follow the header.
    pub eh_max: u16,

    /// Depth of this extent node in the extent tree.
    /// 0 = this extent node points to data blocks;
    /// otherwise, this extent node points to other extent nodes.
    /// The extent tree can be at most 5 levels deep:
    /// a logical block number can be at most 2^32,
    /// and the smallest n that satisfies 4*(((blocksize - 12)/12)^n) >= 2^32 is 5.
    pub eh_depth: u16,

    /// Generation of the tree. (Used by Lustre, but not standard ext4).
    pub eh_generation: u32,
}


#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentIndex {
    /// This index node covers file blocks from ‘block’ onward.
    pub first_block: u32,

    /// Lower 32-bits of the block number of the extent node that is
    /// the next level lower in the tree. The tree node pointed to
    /// can be either another internal node or a leaf node, described below.
    pub ei_leaf_lo: u32,

    /// Upper 16-bits of the previous field.
    pub ei_leaf_hi: u16,

    pub ei_unused: u16,
}




#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4Extent {
    /// First file block number that this extent covers.
    pub first_block: u32,

    /// Number of blocks covered by extent.
    /// If the value of this field is <= 32768, the extent is initialized.
    /// If the value of the field is > 32768, the extent is uninitialized
    /// and the actual extent length is ee_len - 32768.
    /// Therefore, the maximum length of a initialized extent is 32768 blocks,
    /// and the maximum length of an uninitialized extent is 32767.
    pub ee_len: u16,

    /// Upper 16-bits of the block number to which this extent points.
    pub ee_start_hi: u16,

    /// Lower 32-bits of the block number to which this extent points.
    pub ee_start_lo: u32,
}


#[derive(Default, Debug)]
// A single block descriptor
pub struct Ext4Block {
    // Logical block ID
    pub lb_id: u64,

    pub db_id :u64,
    // Buffer
    // buf: Ext4Buf,
    // Data buffer
    pub data: Vec<u8>,
}

/**
 * Linked list directory entry structure
 */
#[derive(Debug)]
pub struct Ext4DirEn {
    pub inode: u32,     // I-node for the entry
    pub entry_len: u16, // Distance to the next directory entry
    pub name_len: u8,   // Lower 8 bits of name length
    pub name_length_high: u8, // Internal fields
    pub name: [u8; 255],      // Entry name
}

pub struct Ext4FakeDirEntry {
	inode: u32,
	entry_length: u16,
	name_length: u8,
	inode_type: u8,
}


/// 文件描述符
pub struct Ext4File {
    /// 挂载点句柄
    pub mp: Ext4MountPoint,
    /// 文件 inode id
    pub inode: u32,
    /// 打开标志
    pub flags: u32,
    /// 文件大小
    pub fsize: u64,
    /// 实际文件位置
    pub fpos: u64,
}



pub  struct Ext4DirSearchResult {
    // block: Ext4Block,
    pub dentry: Ext4DirEn,
}


#[derive(Debug)]
pub struct Ext4ExtentPath {
    // Physical block number
    pub p_block: ext4_fsblk_t,
    // Single block descriptor
    pub block: Ext4Block,
    // Depth of this extent node
    pub depth: u16,
    // Max depth of the extent tree
    pub maxdepth: i32,
    // Pointer to the extent header
    pub header: *const Ext4ExtentHeader,
    // Pointer to the index in the current node
    pub index: *const Ext4ExtentIndex,
    // Pointer to the extent in the current node
    pub extent: *const Ext4Extent,
}



bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct GroupFlags: u16 {
        /// inode table and bitmap are not initialized
        const INODE_UNINIT = 0x1;
        /// block bitmap is not initialized
        const BLOCK_UNINIT = 0x2;
        /// inode table is zeroed
        const INODE_ZEROED = 0x4;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct FileMode: u16 {
        // S_IFMT是文件类型的掩码
        const S_IFMT   = 0xF000; // 文件类型的位域
        // S_IFSOCK是套接字类型
        const S_IFSOCK = 0xC000; // 套接字
        // S_IFLNK是符号链接类型
        const S_IFLNK  = 0xA000; // 符号链接
        // S_IFREG是普通文件类型
        const S_IFREG  = 0x8000; // 普通文件
        // S_IFBLK是块设备类型
        const S_IFBLK  = 0x6000; // 块设备
        // S_IFDIR是目录类型
        const S_IFDIR  = 0x4000; // 目录
        // S_IFCHR是字符设备类型
        const S_IFCHR  = 0x2000; // 字符设备
        // S_IFIFO是FIFO类型
        const S_IFIFO  = 0x1000; // FIFO
        // S_ISUID是设置用户ID位
        const S_ISUID  = 0x0800; // 设置用户ID位
        // S_ISGID是设置组ID位
        const S_ISGID  = 0x0400; // 设置组ID位
        // S_ISVTX是粘滞位
        const S_ISVTX  = 0x0200; // 粘滞位
        // S_IRWXU是用户权限的掩码
        const S_IRWXU  = 0x01E0; // 用户权限的位域
        // S_IRUSR是用户可读权限
        const S_IRUSR  = 0x0100; // 用户可读
        // S_IWUSR是用户可写权限
        const S_IWUSR  = 0x0080; // 用户可写
        // S_IXUSR是用户可执行权限
        const S_IXUSR  = 0x0040; // 用户可执行
        // S_IRWXG是组权限的掩码
        const S_IRWXG  = 0x001C; // 组权限的位域
        // S_IRGRP是组可读权限
        const S_IRGRP  = 0x0010; // 组可读
        // S_IWGRP是组可写权限
        const S_IWGRP  = 0x0008; // 组可写
        // S_IXGRP是组可执行权限
        const S_IXGRP  = 0x0004; // 组可执行
        // S_IRWXO是其他用户权限的掩码
        const S_IRWXO  = 0x0007; // 其他用户权限的位域
        // S_IROTH是其他用户可读权限
        const S_IROTH  = 0x0004; // 其他用户可读
        // S_IWOTH是其他用户可写权限
        const S_IWOTH  = 0x0002; // 其他用户可写
        // S_IXOTH是其他用户可执行权限
        const S_IXOTH  = 0x0001; // 其他用户可执行
    }

    #[derive(Debug)]
    pub struct IFlags: u32 {
        /// Secure deletion
        const Ext4SecrmFl = 0x00000001;
        /// Undelete
        const Ext4UnrmFl = 0x00000002;
        /// Compress file
        const Ext4ComprFl = 0x00000004;
        /// Synchronous updates
        const Ext4SyncFl = 0x00000008;
        /// Immutable file
        const Ext4ImmutableFl = 0x00000010;
        /// writes to file may only append
        const Ext4AppendFl = 0x00000020;
        /// do not dump file
        const Ext4NodumpFl = 0x00000040;
        /// do not update atime
        const Ext4NoatimeFl = 0x00000080;
        ///
        const Ext4DirtyFl = 0x00000100;
        /// One or more compressed clusters
        const Ext4ComprblkFl = 0x00000200;
        /// Don't compress
        const Ext4NocomprFl = 0x00000400;
        /// encrypted file
        const Ext4EncryptFl = 0x00000800;
        /// hash-indexed directory
        const Ext4IndexFl = 0x00001000;
        /// AFS directory
        const Ext4ImagicFl = 0x00002000;
        /// file data should be journaled
        const Ext4JournalDataFl = 0x00004000;
        /// file tail should not be merged
        const Ext4NotailFl = 0x00008000;
        /// dirsync behaviour (directories only)
        const Ext4DirsyncFl = 0x00010000;
        /// Top of directory hierarchies
        const Ext4TopdirFl = 0x00020000;
        /// Set to each huge file
        const Ext4HugeFileFl = 0x00040000;
        /// Inode uses extents
        const Ext4ExtentsFl = 0x00080000;
        /// Verity protected inode
        const Ext4VerityFl = 0x00100000;
        /// Inode used for large EA
        const Ext4EaInodeFl = 0x00200000;
        /// Inode is DAX
        const Ext4DaxFl = 0x02000000;
        /// Inode has inline data.
        const Ext4InlineDataFl = 0x10000000;
        /// Create with parents projid
        const Ext4ProjinheritFl = 0x20000000;
        /// Casefolded directory
        const Ext4CasefoldFl = 0x40000000;
        /// reserved for ext4 lib
        const Ext4ReservedFl = 0x80000000;
        /// User modifiable flags
        const Ext4FlUserModifiable = 0x604BC0FF;
        /// User visible flags
        const Ext4FlUserVisible = 0x705BDFFF;
    }
}

bitflags! {
    #[derive(PartialEq, Debug)]
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
    #[derive(PartialEq, Eq)]
    pub struct DirEntryType: u8 {
        const UNKNOWN = 0;
        const REG_FILE = 1;
        const DIR = 2;
        const CHRDEV = 3;
        const BLKDEV = 4;
        const FIFO = 5;
        const SOCK = 6;
        const SYMLINK = 7;
    }
}


// open/fcntl.
pub const O_ACCMODE: u32 = 0x0003;
pub const O_RDONLY: u32 = 0x00;
pub const O_WRONLY: u32 = 0x01;
pub const O_RDWR: u32 = 0x02;
pub const O_CREAT: u32 = 0x0100; // Not fcntl.
pub const O_EXCL: u32 = 0x0200; // Not fcntl.
pub const O_NOCTTY: u32 = 0x0400; // Not fcntl.
pub const O_TRUNC: u32 = 0x01000; // Not fcntl.
pub const O_APPEND: u32 = 0x02000;
pub const O_NONBLOCK: u32 = 0x04000;
pub const O_NDELAY: u32 = O_NONBLOCK;
pub const O_SYNC: u32 = 0x04010000;
pub const O_FSYNC: u32 = O_SYNC;
pub const O_ASYNC: u32 = 0x020000;
pub const __O_LARGEFILE: u32 = 0x0100000;
pub const __O_DIRECTORY: u32 = 0x0200000;
pub const __O_NOFOLLOW: u32 = 0x0400000;
pub const __O_CLOEXEC: u32 = 0x02000000;
pub const __O_DIRECT: u32 = 0x040000;
pub const __O_NOATIME: u32 = 0x01000000;
pub const __O_PATH: u32 = 0x010000000;
pub const __O_DSYNC: u32 = 0x010000;
pub const __O_TMPFILE: u32 = 0x020000000 | __O_DIRECTORY;


pub const EXT4_INODE_FLAG_EXTENTS:u32 =  0x00080000;   /* Inode uses extents */
pub const EXT4_EXTENT_MAGIC:u16 =  0xF30A;


pub const BASE_OFFSET: u64 = 1024; // 超级块的偏移量
pub const BLOCK_SIZE: u64 = 4096; // 块大小
pub const INODE_SIZE: u64 = 128; // inode大小
pub const ROOT_INODE: u64 = 2; // 根目录的inode号
pub type ext4_lblk_t = u32;
pub type ext4_fsblk_t = u64;


// 定义ext4文件系统的常量
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: usize = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: usize = 64;
pub const EXT4_MIN_BLOCK_SIZE: usize = 1_024; // 1 KiB
pub const EXT4_MAX_BLOCK_SIZE: usize = 65_536; // 64 KiB
pub const EXT4_REV0_INODE_SIZE: usize = 128;
pub const EXT4_INODE_BLOCK_SIZE: usize = 512;
pub const EXT4_INODE_DIRECT_BLOCK_COUNT: usize = 12;
pub const EXT4_INODE_INDIRECT_BLOCK: usize = EXT4_INODE_DIRECT_BLOCK_COUNT;
pub const EXT4_INODE_DOUBLE_INDIRECT_BLOCK: usize = EXT4_INODE_INDIRECT_BLOCK + 1;
pub const EXT4_INODE_TRIPPLE_INDIRECT_BLOCK: usize = EXT4_INODE_DOUBLE_INDIRECT_BLOCK + 1;
pub const EXT4_INODE_BLOCKS: usize = EXT4_INODE_TRIPPLE_INDIRECT_BLOCK + 1;
pub const EXT4_INODE_INDIRECT_BLOCK_COUNT: usize =
    EXT4_INODE_BLOCKS - EXT4_INODE_DIRECT_BLOCK_COUNT;


pub const EXT4_INODE_MODE_FIFO: usize =  0x1000;
pub const EXT4_INODE_MODE_CHARDEV: usize =  0x2000;
pub const EXT4_INODE_MODE_DIRECTORY: usize =  0x4000;
pub const EXT4_INODE_MODE_BLOCKDEV: usize =  0x6000;
pub const EXT4_INODE_MODE_FILE: u16 =  0x8000;
pub const EXT4_INODE_MODE_SOFTLINK: usize =  0xA000;
pub const EXT4_INODE_MODE_SOCKET: usize =  0xC000;
pub const EXT4_INODE_MODE_TYPE_MASK: u16 =  0xF000;
pub const EXT4_SUPERBLOCK_OS_LINUX:usize = 0;
pub const EXT4_SUPERBLOCK_OS_HURD:usize = 1;

pub fn ext4_fs_correspond_inode_mode(file_type: u16) -> InodeMode {
    let filetype = FileMode::from_bits(file_type as u16).unwrap();
    // 使用match语句匹配文件类型
    match filetype {
        FileMode::S_IFDIR => InodeMode::S_IFDIR,
        FileMode::S_IFREG => InodeMode::S_IFREG,
        FileMode::S_IFLNK => InodeMode::S_IFLNK,
        FileMode::S_IFCHR => InodeMode::S_IFCHR,
        FileMode::S_IFBLK => InodeMode::S_IFBLK,
        FileMode::S_IFIFO => InodeMode::S_IFIFO,
        FileMode::S_IFSOCK => InodeMode::S_IFSOCK,

        _ => InodeMode::S_IFREG,
    }
}


impl Ext4Inode{
    pub fn ext4_inode_set_flags(&mut self, f: u32){
        self.flags |= f;
    }

    pub fn ext4_inode_set_mode(&mut self, mode:u16){
        self.mode |= mode;
    }

    pub fn ext4_inode_set_links_cnt(&mut self, cnt:u16){
        self.links_count = cnt;
    }

    pub fn ext4_inode_set_uid(&mut self, uid:u16){
        self.uid = uid;
    }   

	pub fn ext4_inode_set_gid(&mut self, gid:u16){
        self.gid = gid;
    }

	pub fn ext4_inode_set_size(&mut self, size:u32){
        self.size = size;
    }

	pub fn ext4_inode_set_access_time(&mut self, access_time:u32){
        self.atime = access_time;
    }

	pub fn ext4_inode_set_change_inode_time(&mut self, change_inode_time:u32){
        self.ctime = change_inode_time;
    }

	pub fn ext4_inode_set_modif_time(&mut self, modif_time:u32){
        self.mtime = modif_time;
    }

	pub fn ext4_inode_set_del_time(&mut self, del_time:u32){
        self.dtime = del_time;
    }

	pub fn ext4_inode_set_blocks_count(&mut self, blocks_count:u32){
        self.blocks = blocks_count;
    }   

    pub fn ext4_inode_set_generation(&mut self, generation: u32){
        self.generation = generation;
    }

    pub fn ext4_inode_set_csum(&mut self, checksum: u32, inode_size: u16){

        self.osd2.l_i_checksum_lo = ((checksum << 16) >> 16)  as u16;

        if inode_size > 128{
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }

}


impl Ext4ExtentHeader {
    pub fn from_bytes_u32(bytes: &[u32]) -> Ext4ExtentHeader {
        let size = size_of::<Self>();
        let src = bytes.as_ptr() as *const Self;
        let mut dst = Self {
            eh_magic: 0,
            eh_entries: 0,
            eh_max: 0,
            eh_depth: 0,
            eh_generation: 0,
        };
        let ptr = &mut dst as *mut Ext4ExtentHeader as *mut Ext4ExtentHeader;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }

}

impl Ext4DirEntry{
    pub fn from_bytes_offset(bytes: &[u8], offset: usize) -> Ext4DirEntry {
        let new_bytes = &bytes[offset..];
        let size = size_of::<Self>();
        let src = new_bytes.as_ptr() as *const Self;
        let mut dst = Self {
            inode: 0,
            rec_len: 0,
            name_len: 0,
            file_type: 0,
            name: [0; 255],
        };
        let ptr = &mut dst as *mut Ext4DirEntry as *mut Ext4DirEntry;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }
}

impl Ext4ExtentIndex{
    pub fn from_bytes_u32(bytes: &[u32]) -> Ext4ExtentIndex {
        let size = size_of::<Self>();
        let src = bytes.as_ptr() as *const Self;
        let mut dst = Self {
            first_block: 0,
            ei_leaf_lo: 0,
            ei_leaf_hi: 0,
            ei_unused: 0,
        };
        let ptr = &mut dst as *mut Ext4ExtentIndex as *mut Ext4ExtentIndex;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }
}

impl Ext4Extent{
    pub fn from_bytes_u32(bytes: &[u32]) -> Ext4Extent {
        let size = size_of::<Self>();
        let src = bytes.as_ptr() as *const Self;
        let mut dst = Self {
            first_block: 0,
            ee_len: 0,
            ee_start_hi: 0,
            ee_start_lo: 0,
        };
        let ptr = &mut dst as *mut Ext4Extent as *mut Ext4Extent;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }
}

impl Default for Ext4ExtentPath {
    fn default() -> Self {
        Self {
            p_block: 0,
            block: Ext4Block::default(),
            depth: 0,
            maxdepth: 0,
            header: core::ptr::null_mut(),
            index: core::ptr::null_mut(),
            extent: core::ptr::null_mut(),
        }
    }
}


impl Ext4MountPoint {
    pub fn new(name: &str) -> Self {
        let name_string = name.to_string();
        let mut arr: [char; 33] = ['0'; 33];
        for (i, c) in name.chars().enumerate() {
            if i >= arr.len() {
                break;
            }
            arr[i] = c;
        }
        Ext4MountPoint {
            mounted: true,
            mount_name: arr,
            mount_name_string: name_string,
        }
    }
}

impl Default for Ext4DirEn {
    fn default() -> Self {
        Self {
            inode: 0,     // I-node for the entry
            entry_len: 0, // Distance to the next directory entry
            name_len: 0,  // Lower 8 bits of name length
            name_length_high: 0,
            name: [0u8; 255],
        }
    }


}
impl Default for Ext4DirSearchResult {
    fn default() -> Self {
        Self {
            dentry: Ext4DirEn::default(),
        }
    }
}

impl Default for Ext4DirEntry{
    fn default() -> Self {
        
        Self{
            inode: 0,
            rec_len: 0,
            name_len: 0,
            file_type: 0,
            name: [0u8; 255],
        }
    }
}

impl Ext4File{
    pub fn new(mp: Ext4MountPoint) -> Self{

        Self{
            mp: mp,
            inode: 0,
            flags: 0,
            fsize: 0,
            fpos: 0,
        }
    }
}
pub trait Ext4Traits {
    fn read_block(offset: u64) ->Vec<u8>;

    fn write_block(offset: u64, buf: &[u8]);
}


#[derive(Debug)]
// A single block descriptor
pub struct Ext4BlockNew<'a, A:Ext4Traits> {
    // disk block id
    pub disk_block_id: u64,

    // size BLOCK_SIZE
    pub block_data: &'a mut Vec<u8>,

    pub dirty: bool,

    pub phantom: PhantomData<A>,
}


impl <'a, A:Ext4Traits>Ext4BlockNew<'a, A> {
    
    pub fn write_back(&mut self){

        let data :&[u8] = self.block_data;
        A::write_block(self.disk_block_id * BLOCK_SIZE, data);
    }


    pub fn sync(&mut self){
        if self.dirty{
            self.write_back();

        }
    }
}











// 定义ext4_inode这个结构体
// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
struct Ext4Inodenew {
    i_mode: u16, // 文件模式
    i_uid: u16, // 所有者Uid的低16位
    i_size_lo: u32, // 文件大小（字节）
    i_atime: u32, // 访问时间
    i_ctime: u32, // Inode修改时间
    i_mtime: u32, // 文件修改时间
    i_dtime: u32, // 删除时间
    i_gid: u16, // 组Id的低16位
    i_links_count: u16, // 链接数
    i_blocks_lo: u32, // 块数
    i_flags: u32, // 文件标志
    osd1: Linux1, // 操作系统相关的字段1
    i_block: [u32; 15], // 指向块的指针
    i_generation: u32, // 文件版本（用于NFS）
    i_file_acl_lo: u32, // 文件ACL
    i_size_high: u32,
    i_obso_faddr: u32, // 废弃的片段地址
    
    osd2: Linux2, // 操作系统相关的字段2

    i_extra_isize: u16,
    i_checksum_hi: u16, // crc32c(uuid+inum+inode) BE
    i_ctime_extra: u32, // 额外的修改时间（nsec << 2 | epoch）
    i_mtime_extra: u32, // 额外的文件修改时间（nsec << 2 | epoch）
    i_atime_extra: u32, // 额外的访问时间（nsec << 2 | epoch）
    i_crtime: u32, // 文件创建时间
    i_crtime_extra: u32, // 额外的文件创建时间（nsec << 2 | epoch）
    i_version_hi: u32, // 64位版本的高32位
    i_projid: u32, // 项目ID
}

// // 定义Osd1这个共用体
// #[repr(C)]
// #[derive(Clone, Copy)]
// union Osd1 {
//     linux1: Linux1,
//     hurd1: Hurd1,
//     masix1: Masix1,
// }

// 定义Linux1这个结构体
#[repr(C)]
struct Linux1 {
    l_i_version: u32,
}

// // 定义Hurd1这个结构体
// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// struct Hurd1 {
//     h_i_translator: u32,
// }

// // 定义Masix1这个结构体
// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// struct Masix1 {
//     m_i_reserved1: u32,
// }

// // 定义Osd2这个共用体
// #[repr(C)]
// #[derive(Clone, Copy)]
// union Osd2 {
//     linux2: Linux2,
//     hurd2: Hurd2,
//     masix2: Masix2,
// }

// 定义Linux2这个结构体
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Linux2 {
    pub l_i_blocks_high: u16, // 原来是l_i_reserved1
    pub l_i_file_acl_high: u16,
    pub l_i_uid_high: u16, // 这两个字段
    pub l_i_gid_high: u16, // 原来是reserved2[0]
    pub l_i_checksum_lo: u16, // crc32c(uuid+inum+inode) LE
    pub l_i_reserved: u16,
}

// // 定义Hurd2这个结构体
// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// struct Hurd2 {
//     h_i_reserved1: u16, // 在ext4中移除的废弃的片段号/大小
//     h_i_mode_high: u16,
//     h_i_uid_high: u16,
//     h_i_gid_high: u16,
//     h_i_author: u32,
// }

// // 定义Masix2这个结构体
// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// struct Masix2 {
//     h_i_reserved1: u16, // 在ext4中移除的废弃的片段号/大小
//     m_i_file_acl_high: u16,
//     m_i_reserved2: [u32; 2],
// }


// 假设我们已经导入了一些必要的模块，如std::mem, std::ptr, std::slice等

// ext4_buf结构
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ext4_buf {
    // pub flags: i32, // 标志，表示缓冲区的状态，如是否锁定，是否脏等
    pub block: ext4_fsblk_t, // 块号
    pub lba: u64, // 逻辑块地址，表示缓冲区对应的磁盘块号
    pub data: Vec<u8>, // 使用Vec来存储数据
    // pub lru_prio: u32, // 优先级，表示缓冲区在最近最少使用（LRU）算法中的优先级
    // pub lru_id: u32, // 缓冲区的唯一标识符，用于在LRU列表中查找
    pub bc: *mut ext4_bcache, // 缓存指针，指向缓冲区所属的缓存结构
    pub dirty: bool, // 脏标志，表示缓冲区是否需要写回磁盘
    // lba_node: ext4_buf__bindgen_ty_1, // LBA节点，用于将缓冲区链接到以LBA为键的哈希表中
    // lru_node: ext4_buf__bindgen_ty_2, // LRU节点，用于将缓冲区链接到LRU列表中
    // dirty_node: ext4_buf__bindgen_ty_3, // 脏节点，用于将缓冲区链接到脏列表中
    // pub end_write: Option<unsafe extern "C" fn(buf: *mut ext4_buf)>, // 写回回调函数，用于在缓冲区写回磁盘后执行一些操作
}

impl ext4_buf{
    // ext4_buf的new函数
    pub fn new(block: ext4_fsblk_t) -> *mut ext4_buf {

        // 分配一块内存来存储ext4_buf结构
        let b = Box::new(ext4_buf {
            data: vec![0u8; 4096],
            block: block,
            lba: 0,
            bc: ext4_bcache::new(),
            dirty :false,
        });

        // 返回ext4_buf结构的指针
        return Box::into_raw(b);
    }
}



// ext4_bcache结构
#[repr(C)]
pub struct ext4_bcache {
    // size: u32, // 缓存大小，以字节为单位
    // block_size: u32, // 缓存块大小，以字节为单位
    // blocks: u32, // 缓存块数
    cache: Vec<u8>, // 缓存列表，指向缓存块的数组
    // hash_table: *mut ext4_buf, // 哈希表，指向以逻辑块地址为键的哈希表
    // lru_list: *mut ext4_buf, // LRU列表，指向最近最少使用（LRU）算法的双向链表
    // dirty_list: *mut ext4_buf, // 脏列表，指向需要写回磁盘的缓冲区的双向链表
    lru_id: u32, // LRU标识符，用于给缓冲区分配唯一的ID
    // lock: ext4_bcache__bindgen_ty_1, // 锁，用于保护缓存的并发访问
}

impl ext4_bcache {

    // ext4_bcache的new函数
    fn new() -> *mut ext4_bcache {
        // 分配一块内存来存储ext4_bcache结构
        let bc = Box::new(ext4_bcache {
            // size: size,
            // block_size: block_size,
            // blocks: blocks,
            cache: vec![0u8; BLOCK_SIZE as usize], // 使用vec来分配并初始化缓存列表的内容
            // hash_table: core::ptr::null_mut(),
            // lru_list: ptr::null_mut(),
            // dirty_list: ptr::null_mut(),
            lru_id: 0,
            // lock: ext4_bcache__bindgen_ty_1::new(), // 调用锁的new函数来初始化锁
        });

        // 返回ext4_bcache结构的指针
        return Box::into_raw(bc);
    }

    
}