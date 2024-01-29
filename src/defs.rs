use bitflags::bitflags;
use core::mem::size_of;

use crate::defs::*;
use crate::ext4::*;



#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    Start(usize),
    End(isize),
    Current(isize),
}

/// Maximum bytes in a path
pub const PATH_MAX: usize = 4096;

/// Maximum bytes in a file name
pub const NAME_MAX: usize = 255;

/// The upper limit for resolving symbolic links
pub const SYMLINKS_MAX: usize = 40;

pub type CStr256 = FixedCStr<256>;
pub type Str16 = FixedStr<16>;
pub type Str64 = FixedStr<64>;

/// An owned C-compatible string with a fixed capacity of `N`.
///
/// The string is terminated with a null byte.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Pod)]
pub struct FixedCStr<const N: usize>([u8; N]);

impl<const N: usize> FixedCStr<N> {
    pub fn len(&self) -> usize {
        self.0.iter().position(|&b| b == 0).unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_str(&self) -> Result<&str> {
        Ok(alloc::str::from_utf8(self.as_bytes())?)
    }

    pub fn as_cstr(&self) -> Result<&CStr> {
        Ok(CStr::from_bytes_with_nul(self.as_bytes_with_nul())?)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[0..self.len()]
    }

    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.0[0..=self.len()]
    }
}

impl<'a, const N: usize> From<&'a [u8]> for FixedCStr<N> {
    fn from(bytes: &'a [u8]) -> Self {
        assert!(N > 0);

        let mut inner = [0u8; N];
        let len = {
            let mut nul_byte_idx = match bytes.iter().position(|&b| b == 0) {
                Some(idx) => idx,
                None => bytes.len(),
            };
            if nul_byte_idx >= N {
                nul_byte_idx = N - 1;
            }
            nul_byte_idx
        };
        inner[0..len].copy_from_slice(&bytes[0..len]);
        Self(inner)
    }
}

impl<'a, const N: usize> From<&'a str> for FixedCStr<N> {
    fn from(string: &'a str) -> Self {
        let bytes = string.as_bytes();
        Self::from(bytes)
    }
}

impl<'a, const N: usize> From<&'a CStr> for FixedCStr<N> {
    fn from(cstr: &'a CStr) -> Self {
        let bytes = cstr.to_bytes_with_nul();
        Self::from(bytes)
    }
}

impl<const N: usize> Default for FixedCStr<N> {
    fn default() -> Self {
        Self([0u8; N])
    }
}

impl<const N: usize> Debug for FixedCStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.as_cstr() {
            Ok(cstr) => write!(f, "{:?}", cstr),
            Err(_) => write!(f, "{:?}", self.as_bytes()),
        }
    }
}

/// An owned string with a fixed capacity of `N`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Pod)]
pub struct FixedStr<const N: usize>([u8; N]);

impl<const N: usize> FixedStr<N> {
    pub fn len(&self) -> usize {
        self.0.iter().position(|&b| b == 0).unwrap_or(N)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_str(&self) -> Result<&str> {
        Ok(alloc::str::from_utf8(self.as_bytes())?)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[0..self.len()]
    }
}

impl<'a, const N: usize> From<&'a [u8]> for FixedStr<N> {
    fn from(bytes: &'a [u8]) -> Self {
        let mut inner = [0u8; N];
        let len = {
            let mut nul_byte_idx = match bytes.iter().position(|&b| b == 0) {
                Some(idx) => idx,
                None => bytes.len(),
            };
            if nul_byte_idx > N {
                nul_byte_idx = N;
            }
            nul_byte_idx
        };
        inner[0..len].copy_from_slice(&bytes[0..len]);
        Self(inner)
    }
}

impl<'a, const N: usize> From<&'a str> for FixedStr<N> {
    fn from(string: &'a str) -> Self {
        let bytes = string.as_bytes();
        Self::from(bytes)
    }
}

impl<const N: usize> Default for FixedStr<N> {
    fn default() -> Self {
        Self([0u8; N])
    }
}

impl<const N: usize> Debug for FixedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.as_str() {
            Ok(string) => write!(f, "{}", string),
            Err(_) => write!(f, "{:?}", self.as_bytes()),
        }
    }
}

/// 文件描述符
pub struct Ext4FileNew {
    /// 挂载点句柄
    pub mp: *mut Ext4MountPoint,
    /// 文件 inode id
    pub inode: u32,
    /// 打开标志
    pub flags: u32,
    /// 文件大小
    pub fsize: u64,
    /// 实际文件位置
    pub fpos: u64,
}


impl Ext4FileNew{
    pub fn new() -> Self{
        Self{
            mp: core::ptr::null_mut(),
            inode: 0,
            flags: 0,
            fsize: 0,
            fpos: 0,
        }
    }
}


// 结构体表示超级块
#[repr(C)]
pub struct Ext4Superblock {
    inodes_count: u32, // 节点数
    blocks_count_lo: u32, // 块数
    reserved_blocks_count_lo: u32, // 保留块数
    free_blocks_count_lo: u32, // 空闲块数
    free_inodes_count: u32, // 空闲节点数
    first_data_block: u32, // 第一个数据块
    log_block_size: u32, // 块大小
    log_cluster_size: u32, // 废弃的片段大小
    blocks_per_group: u32, // 每组块数
    frags_per_group: u32, // 废弃的每组片段数
    inodes_per_group: u32, // 每组节点数
    mount_time: u32, // 挂载时间
    write_time: u32, // 写入时间
    mount_count: u16, // 挂载次数
    max_mount_count: u16, // 最大挂载次数
    magic: u16, // 魔数，0xEF53
    state: u16, // 文件系统状态
    errors: u16, // 检测到错误时的行为
    minor_rev_level: u16, // 次版本号
    last_check_time: u32, // 最后检查时间
    check_interval: u32, // 检查间隔
    creator_os: u32, // 创建者操作系统
    rev_level: u32, // 版本号
    def_resuid: u16, // 保留块的默认uid
    def_resgid: u16, // 保留块的默认gid

    // 仅适用于EXT4_DYNAMIC_REV超级块的字段
    first_inode: u32, // 第一个非保留节点
    inode_size: u16, // 节点结构的大小
    block_group_index: u16, // 此超级块的块组索引
    features_compatible: u32, // 兼容特性集
    features_incompatible: u32, // 不兼容特性集
    features_read_only: u32, // 只读兼容特性集
    uuid: [u8; 16], // 卷的128位uuid
    volume_name: [u8; 16], // 卷名
    last_mounted: [u8; 64], // 最后挂载的目录
    algorithm_usage_bitmap: u32, // 用于压缩的算法

    // 性能提示。只有当EXT4_FEATURE_COMPAT_DIR_PREALLOC标志打开时，才进行目录预分配
    s_prealloc_blocks: u8, // 尝试预分配的块数
    s_prealloc_dir_blocks: u8, // 为目录预分配的块数
    s_reserved_gdt_blocks: u16, // 在线增长时每组保留的描述符数

    // 如果EXT4_FEATURE_COMPAT_HAS_JOURNAL设置，表示支持日志
    journal_uuid: [u8; 16], // 日志超级块的UUID
    journal_inode_number: u32, // 日志文件的节点号
    journal_dev: u32, // 日志文件的设备号
    last_orphan: u32, // 待删除节点的链表头
    hash_seed: [u32; 4], // HTREE散列种子
    default_hash_version: u8, // 默认的散列版本
    journal_backup_type: u8,
    desc_size: u16, // 组描述符的大小
    default_mount_opts: u32, // 默认的挂载选项
    first_meta_bg: u32, // 第一个元数据块组
    mkfs_time: u32, // 文件系统创建的时间
    journal_blocks: [u32; 17], // 日志节点的备份

    // 如果EXT4_FEATURE_COMPAT_64BIT设置，表示支持64位
    blocks_count_hi: u32, // 块数
    reserved_blocks_count_hi: u32, // 保留块数
    free_blocks_count_hi: u32, // 空闲块数
    min_extra_isize: u16, // 所有节点至少有#字节
    want_extra_isize: u16, // 新节点应该保留#字节
    flags: u32, // 杂项标志
    raid_stride: u16, // RAID步长
    mmp_interval: u16, // MMP检查的等待秒数
    mmp_block: u64, // 多重挂载保护的块
    raid_stripe_width: u32, // 所有数据磁盘上的块数（N * 步长）
    log_groups_per_flex: u8, // FLEX_BG组的大小
    checksum_type: u8,
    reserved_pad: u16,
    kbytes_written: u64, // 写入的千字节数
    snapshot_inum: u32, // 活动快照的节点号
    snapshot_id: u32, // 活动快照的顺序ID
    snapshot_r_blocks_count: u64, // 为活动快照的未来使用保留的块数
    snapshot_list: u32, // 磁盘上快照列表的头节点号
    error_count: u32, // 文件系统错误的数目
    first_error_time: u32, // 第一次发生错误的时间
    first_error_ino: u32, // 第一次发生错误的节点号
    first_error_block: u64, // 第一次发生错误的块号
    first_error_func: [u8; 32], // 第一次发生错误的函数
    first_error_line: u32, // 第一次发生错误的行号
    last_error_time: u32, // 最近一次发生错误的时间
    last_error_ino: u32, // 最近一次发生错误的节点号
    last_error_line: u32, // 最近一次发生错误的行号
    last_error_block: u64, // 最近一次发生错误的块号
    last_error_func: [u8; 32], // 最近一次发生错误的函数
    mount_opts: [u8; 64],
    usr_quota_inum: u32, // 用于跟踪用户配额的节点
    grp_quota_inum: u32, // 用于跟踪组配额的节点
    overhead_clusters: u32, // 文件系统中的开销块/簇
    backup_bgs: [u32; 2], // 有sparse_super2超级块的组
    encrypt_algos: [u8; 4], // 使用的加密算法
    encrypt_pw_salt: [u8; 16], // 用于string2key算法的盐
    lpf_ino: u32, // lost+found节点的位置
    padding: [u32; 100], // 块的末尾的填充
    checksum: u32, // crc32c(superblock)
}


impl TryFrom<Vec<u8>> for Ext4Superblock{
    type Error = u64;
    fn try_from(value: Vec<u8>) -> Result<Self, u64> {
        let data = &value[..size_of::<Ext4Superblock>()];
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4BlockGroup {
    block_bitmap_lo: u32, // 块位图块
    inode_bitmap_lo: u32, // 节点位图块
    inode_table_first_block_lo: u32, // 节点表块
    free_blocks_count_lo: u16, // 空闲块数
    free_inodes_count_lo: u16, // 空闲节点数
    used_dirs_count_lo: u16, // 目录数
    flags: u16, // EXT4_BG_flags (INODE_UNINIT, etc)
    exclude_bitmap_lo: u32, // 快照排除位图
    block_bitmap_csum_lo: u16, // crc32c(s_uuid+grp_num+bbitmap) LE
    inode_bitmap_csum_lo: u16, // crc32c(s_uuid+grp_num+ibitmap) LE
    itable_unused_lo: u16, // 未使用的节点数
    checksum: u16, // crc16(sb_uuid+group+desc)

    block_bitmap_hi: u32, // 块位图块 MSB
    inode_bitmap_hi: u32, // 节点位图块 MSB
    inode_table_first_block_hi: u32, // 节点表块 MSB
    free_blocks_count_hi: u16, // 空闲块数 MSB
    free_inodes_count_hi: u16, // 空闲节点数 MSB
    used_dirs_count_hi: u16, // 目录数 MSB
    itable_unused_hi: u16, // 未使用的节点数 MSB
    exclude_bitmap_hi: u32, // 快照排除位图 MSB
    block_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+bbitmap) BE
    inode_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+ibitmap) BE
    reserved: u32, // 填充
}



pub struct Inode{
    ino: u32,
    block_group_idx: usize,
    inner: Inner,
    fs: Weak<Ext4>,
}

impl Inode{
    pub fn fs(&self) -> Arc<Ext4> {
        self.fs.upgrade().unwrap()
    }
}

struct Inner {
    inode: Ext4Inode,
    weak_self: Weak<Inode>,
}

impl Inner{
    pub fn inode(&self) -> Arc<Ext4Inode> {
        self.weak_self.upgrade().unwrap()
    }
}

