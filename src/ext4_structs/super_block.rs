use super::*;
use crate::consts::*;
use crate::prelude::*;
use crate::utils::*;
use crate::BlockDevice;
use core::mem::size_of;

use crate::BASE_OFFSET;

// 结构体表示超级块
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Superblock {
    pub inodes_count: u32,         // 节点数
    blocks_count_lo: u32,          // 块数
    reserved_blocks_count_lo: u32, // 保留块数
    free_blocks_count_lo: u32,     // 空闲块数
    free_inodes_count: u32,        // 空闲节点数
    pub first_data_block: u32,     // 第一个数据块
    log_block_size: u32,           // 块大小
    log_cluster_size: u32,         // 废弃的片段大小
    blocks_per_group: u32,         // 每组块数
    frags_per_group: u32,          // 废弃的每组片段数
    pub inodes_per_group: u32,     // 每组节点数
    mount_time: u32,               // 挂载时间
    write_time: u32,               // 写入时间
    mount_count: u16,              // 挂载次数
    max_mount_count: u16,          // 最大挂载次数
    magic: u16,                    // 魔数，0xEF53
    state: u16,                    // 文件系统状态
    errors: u16,                   // 检测到错误时的行为
    minor_rev_level: u16,          // 次版本号
    last_check_time: u32,          // 最后检查时间
    check_interval: u32,           // 检查间隔
    pub creator_os: u32,           // 创建者操作系统
    pub rev_level: u32,                // 版本号
    def_resuid: u16,               // 保留块的默认uid
    def_resgid: u16,               // 保留块的默认gid

    // 仅适用于EXT4_DYNAMIC_REV超级块的字段
    first_inode: u32,            // 第一个非保留节点
    pub inode_size: u16,         // 节点结构的大小
    block_group_index: u16,      // 此超级块的块组索引
    features_compatible: u32,    // 兼容特性集
    features_incompatible: u32,  // 不兼容特性集
    pub features_read_only: u32, // 只读兼容特性集
    pub uuid: [u8; 16],          // 卷的128位uuid
    volume_name: [u8; 16],       // 卷名
    last_mounted: [u8; 64],      // 最后挂载的目录
    algorithm_usage_bitmap: u32, // 用于压缩的算法

    // 性能提示。只有当EXT4_FEATURE_COMPAT_DIR_PREALLOC标志打开时，才进行目录预分配
    s_prealloc_blocks: u8,      // 尝试预分配的块数
    s_prealloc_dir_blocks: u8,  // 为目录预分配的块数
    s_reserved_gdt_blocks: u16, // 在线增长时每组保留的描述符数

    // 如果EXT4_FEATURE_COMPAT_HAS_JOURNAL设置，表示支持日志
    journal_uuid: [u8; 16],    // 日志超级块的UUID
    journal_inode_number: u32, // 日志文件的节点号
    journal_dev: u32,          // 日志文件的设备号
    last_orphan: u32,          // 待删除节点的链表头
    hash_seed: [u32; 4],       // HTREE散列种子
    default_hash_version: u8,  // 默认的散列版本
    journal_backup_type: u8,
    pub desc_size: u16,        // 组描述符的大小
    default_mount_opts: u32,   // 默认的挂载选项
    first_meta_bg: u32,        // 第一个元数据块组
    mkfs_time: u32,            // 文件系统创建的时间
    journal_blocks: [u32; 17], // 日志节点的备份

    // 如果EXT4_FEATURE_COMPAT_64BIT设置，表示支持64位
    blocks_count_hi: u32,          // 块数
    reserved_blocks_count_hi: u32, // 保留块数
    free_blocks_count_hi: u32,     // 空闲块数
    min_extra_isize: u16,          // 所有节点至少有#字节
    want_extra_isize: u16,         // 新节点应该保留#字节
    flags: u32,                    // 杂项标志
    raid_stride: u16,              // RAID步长
    mmp_interval: u16,             // MMP检查的等待秒数
    mmp_block: u64,                // 多重挂载保护的块
    raid_stripe_width: u32,        // 所有数据磁盘上的块数（N * 步长）
    log_groups_per_flex: u8,       // FLEX_BG组的大小
    checksum_type: u8,
    reserved_pad: u16,
    kbytes_written: u64,          // 写入的千字节数
    snapshot_inum: u32,           // 活动快照的节点号
    snapshot_id: u32,             // 活动快照的顺序ID
    snapshot_r_blocks_count: u64, // 为活动快照的未来使用保留的块数
    snapshot_list: u32,           // 磁盘上快照列表的头节点号
    error_count: u32,             // 文件系统错误的数目
    first_error_time: u32,        // 第一次发生错误的时间
    first_error_ino: u32,         // 第一次发生错误的节点号
    first_error_block: u64,       // 第一次发生错误的块号
    first_error_func: [u8; 32],   // 第一次发生错误的函数
    first_error_line: u32,        // 第一次发生错误的行号
    last_error_time: u32,         // 最近一次发生错误的时间
    last_error_ino: u32,          // 最近一次发生错误的节点号
    last_error_line: u32,         // 最近一次发生错误的行号
    last_error_block: u64,        // 最近一次发生错误的块号
    last_error_func: [u8; 32],    // 最近一次发生错误的函数
    mount_opts: [u8; 64],
    usr_quota_inum: u32,       // 用于跟踪用户配额的节点
    grp_quota_inum: u32,       // 用于跟踪组配额的节点
    overhead_clusters: u32,    // 文件系统中的开销块/簇
    backup_bgs: [u32; 2],      // 有sparse_super2超级块的组
    encrypt_algos: [u8; 4],    // 使用的加密算法
    encrypt_pw_salt: [u8; 16], // 用于string2key算法的盐
    lpf_ino: u32,              // lost+found节点的位置
    padding: [u32; 100],       // 块的末尾的填充
    checksum: u32,             // crc32c(superblock)
}

impl TryFrom<Vec<u8>> for Ext4Superblock {
    type Error = u64;
    fn try_from(value: Vec<u8>) -> core::result::Result<Self, u64> {
        let data = &value[..size_of::<Ext4Superblock>()];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl Ext4Superblock {
    pub fn sync_super_block_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        block_device.write_offset(BASE_OFFSET, data);
    }
}

impl Ext4Superblock {
    /// Returns the size of inode structure.
    pub fn inode_size(&self) -> u16 {
        self.inode_size
    }

    /// Returns the size of inode structure.
    pub fn inode_size_file(&self, inode: &Ext4Inode) -> u64 {
        let mode = inode.mode;

        // 获取inode的低32位大小
        let mut v = inode.size as u64;
        // 如果文件系统的版本号大于0，并且inode的类型是文件
        if self.rev_level > 0 && (mode & EXT4_INODE_MODE_TYPE_MASK) == EXT4_INODE_MODE_FILE as u16 {
            // 获取inode的高32位大小，并左移32位
            let hi = (inode.size_hi as u64) << 32;
            // 用或运算符将低32位和高32位拼接为一个u64值
            v |= hi;
        }

        // 返回inode的大小
        v
    }

    pub fn free_inodes_count(&self) -> u32 {
        self.free_inodes_count
    }

    /// Returns total number of inodes.
    pub fn total_inodes(&self) -> u32 {
        self.inodes_count
    }

    /// Returns the number of blocks in each block group.
    pub fn blocks_per_group(&self) -> u32 {
        self.blocks_per_group
    }

    /// Returns the size of block.
    pub fn block_size(&self) -> u32 {
        1024 << self.log_block_size
    }

    /// Returns the number of inodes in each block group.
    pub fn inodes_per_group(&self) -> u32 {
        self.inodes_per_group
    }

    /// Returns the number of block groups.
    pub fn block_groups_count(&self) -> u32 {
        let cnt = (((self.blocks_count_hi.to_le() as u64) << 32) as u32 | self.blocks_count_lo)
            / self.blocks_per_group;
        if cnt == 0 {
            1
        } else {
            cnt
        } 

        // cnt
    }

    pub fn blocks_count(&self) -> u32 {
        ((self.blocks_count_hi.to_le() as u64) << 32) as u32 | self.blocks_count_lo
    }

    pub fn desc_size(&self) -> u16 {
        let size = self.desc_size;

        if size < EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            return EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE as u16;
        } else {
            size
        }
    }

    pub fn extra_size(&self) -> u16 {
        self.want_extra_isize
    }

    pub fn get_inodes_in_group_cnt(&self, bgid: u32) -> u32 {
        let block_group_count = self.block_groups_count();
        let inodes_per_group = self.inodes_per_group;

        let total_inodes = ((self.inodes_count as u64) << 32) as u32;
        if bgid < block_group_count - 1 {
            inodes_per_group
        } else {
            total_inodes - ((block_group_count - 1) * inodes_per_group)
        }
    }

    pub fn decrease_free_inodes_count(&mut self) {
        self.free_inodes_count -= 1;
    }

    pub fn free_blocks_count(&self) -> u64 {
        self.free_blocks_count_lo as u64 | ((self.free_blocks_count_hi as u64) << 32).to_le()
    }

    pub fn set_free_blocks_count(&mut self, free_blocks: u64) {
        self.free_blocks_count_lo = ((free_blocks << 32) >> 32) as u32;

        self.free_blocks_count_hi = (free_blocks >> 32) as u32;
    }

    pub fn sync_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        block_device.write_offset(BASE_OFFSET, data);
    }

    pub fn sync_to_disk_with_csum(&mut self, block_device: Arc<dyn BlockDevice>) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        let checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &data,
            0x3fc,
        );

        self.checksum = checksum;
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        block_device.write_offset(BASE_OFFSET, data);
    }

    // pub fn sync_super_block_to_disk(&self, block_device: Arc<dyn BlockDevice>){
    //     let data = unsafe {
    //         core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
    //     };
    //     block_device.write_offset(BASE_OFFSET, data);
    // }
}

#[allow(unused)]
pub fn ext4_ialloc_bitmap_csum(bitmap: &[u8], s: &Ext4Superblock) -> u32 {
    let mut csum = 0;
    let inodes_per_group = s.inodes_per_group;
    let uuid = s.uuid;
    csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
    csum = ext4_crc32c(csum, bitmap, (inodes_per_group + 7) / 8);
    csum
}

#[allow(unused)]
pub fn ext4_balloc_bitmap_csum(bitmap: &[u8], s: &Ext4Superblock) -> u32 {
    let mut csum = 0;
    let blocks_per_group = s.blocks_per_group;
    let uuid = s.uuid;
    csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
    csum = ext4_crc32c(csum, bitmap, (blocks_per_group / 8) as u32);
    csum
}
