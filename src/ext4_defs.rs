use bitflags::bitflags;
use core::mem::size_of;

use super::*;
use crate::prelude::*;

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    Start(usize),
    End(isize),
    Current(isize),
}

/// 文件描述符
pub struct Ext4File {
    /// 挂载点句柄
    pub mp: *mut Ext4MountPoint,
    /// 文件 inode id
    pub inode: u32,
    /// 打开标志
    pub flags: u32,
    /// 文件大小
    pub fsize: u64,
    /// 实际文件位置
    pub fpos: usize,
}

impl Ext4File {
    pub fn new() -> Self {
        Self {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Superblock {
    inodes_count: u32,             // 节点数
    blocks_count_lo: u32,          // 块数
    reserved_blocks_count_lo: u32, // 保留块数
    free_blocks_count_lo: u32,     // 空闲块数
    free_inodes_count: u32,        // 空闲节点数
    first_data_block: u32,         // 第一个数据块
    log_block_size: u32,           // 块大小
    log_cluster_size: u32,         // 废弃的片段大小
    blocks_per_group: u32,         // 每组块数
    frags_per_group: u32,          // 废弃的每组片段数
    inodes_per_group: u32,         // 每组节点数
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
    creator_os: u32,               // 创建者操作系统
    rev_level: u32,                // 版本号
    def_resuid: u16,               // 保留块的默认uid
    def_resgid: u16,               // 保留块的默认gid

    // 仅适用于EXT4_DYNAMIC_REV超级块的字段
    first_inode: u32,            // 第一个非保留节点
    inode_size: u16,             // 节点结构的大小
    block_group_index: u16,      // 此超级块的块组索引
    features_compatible: u32,    // 兼容特性集
    features_incompatible: u32,  // 不兼容特性集
    features_read_only: u32,     // 只读兼容特性集
    uuid: [u8; 16],              // 卷的128位uuid
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
    desc_size: u16,            // 组描述符的大小
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
        (((self.blocks_count_hi.to_le() as u64) << 32) as u32 | self.blocks_count_lo)
            / self.blocks_per_group
    }

    pub fn desc_size(&self) -> u16 {
        let size = self.desc_size;

        if size < EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            return EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE as u16;
        } else {
            size
        }
    }

    pub fn extra_size(&self) -> u16{
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
        self.free_blocks_count_lo = ((free_blocks << 32) >> 32).to_le() as u32;
        self.free_blocks_count_hi = (free_blocks >> 32) as u32;
    }

    pub fn sync_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        block_device.write_offset(BASE_OFFSET, data);
    }

    pub fn sync_to_disk_with_csum(&self, block_device: Arc<dyn BlockDevice>) {
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
    pub faddr: u32,   /* Obsoleted fragment address */
    pub osd2: Linux2, // 操作系统相关的字段2

    pub i_extra_isize: u16,
    pub i_checksum_hi: u16,  // crc32c(uuid+inum+inode) BE
    pub i_ctime_extra: u32,  // 额外的修改时间（nsec << 2 | epoch）
    pub i_mtime_extra: u32,  // 额外的文件修改时间（nsec << 2 | epoch）
    pub i_atime_extra: u32,  // 额外的访问时间（nsec << 2 | epoch）
    pub i_crtime: u32,       // 文件创建时间
    pub i_crtime_extra: u32, // 额外的文件创建时间（nsec << 2 | epoch）
    pub i_version_hi: u32,   // 64位版本的高32位
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Linux2 {
    pub l_i_blocks_high: u16, // 原来是l_i_reserved1
    pub l_i_file_acl_high: u16,
    pub l_i_uid_high: u16,    // 这两个字段
    pub l_i_gid_high: u16,    // 原来是reserved2[0]
    pub l_i_checksum_lo: u16, // crc32c(uuid+inum+inode) LE
    pub l_i_reserved: u16,
}

impl TryFrom<&[u8]> for Ext4Inode {
    type Error = u64;
    fn try_from(data: &[u8]) -> core::result::Result<Self, u64> {
        let data = &data[..size_of::<Ext4Inode>()];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl Ext4Inode {
    pub fn ext4_get_inode_flags(&self) -> u32 {
        self.flags
    }
    pub fn ext4_get_inode_mode(&self) -> u16 {
        self.mode
    }

    pub fn ext4_inode_type(&self, super_block: &Ext4Superblock) -> u32{
        let mut v = self.mode;

        if super_block.creator_os == EXT4_SUPERBLOCK_OS_HURD{
            v |= ((self.osd2.l_i_file_acl_high as u32 ) << 16) as u16;
        }

        (v & EXT4_INODE_MODE_TYPE_MASK) as u32
    }

    pub fn ext4_inode_set_flags(&mut self, f: u32) {
        self.flags |= f;
    }

    pub fn ext4_inode_set_mode(&mut self, mode: u16) {
        self.mode |= mode;
    }

    pub fn ext4_inode_set_links_cnt(&mut self, cnt: u16) {
        self.links_count = cnt;
    }

    pub fn ext4_inode_set_uid(&mut self, uid: u16) {
        self.uid = uid;
    }

    pub fn ext4_inode_set_gid(&mut self, gid: u16) {
        self.gid = gid;
    }

    pub fn ext4_inode_set_size(&mut self, size: u64) {
        self.size = ((size << 32) >> 32)as u32;
        self.size_hi = (size >> 32) as u32;
    }

    pub fn ext4_inode_get_size(&self) -> u64{
        self.size as u64 | ((self.size_hi as u64) << 32)
    }

    pub fn ext4_inode_set_access_time(&mut self, access_time: u32) {
        self.atime = access_time;
    }

    pub fn ext4_inode_set_change_inode_time(&mut self, change_inode_time: u32) {
        self.ctime = change_inode_time;
    }

    pub fn ext4_inode_set_modif_time(&mut self, modif_time: u32) {
        self.mtime = modif_time;
    }

    pub fn ext4_inode_set_del_time(&mut self, del_time: u32) {
        self.dtime = del_time;
    }

    pub fn ext4_inode_set_blocks_count(&mut self, blocks_count: u32) {
        self.blocks = blocks_count;
    }

    pub fn ext4_inode_set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    pub fn ext4_inode_set_extra_isize(&mut self, extra_isize: u16) {
        self.i_extra_isize = extra_isize;
    }

    fn get_checksum(&self, super_block: &Ext4Superblock) -> u32 {
        let inode_size = super_block.inode_size;
        let mut v: u32 = self.osd2.l_i_checksum_lo as u32;
        if inode_size > 128 {
            v |= (self.i_checksum_hi as u32) << 16;
        }
        v
    }

    pub fn set_inode_checksum_value(
        &mut self,
        super_block: &Ext4Superblock,
        inode_id: u32,
        checksum: u32,
    ) {
        let inode_size = super_block.inode_size();

        self.osd2.l_i_checksum_lo = ((checksum << 16) >> 16) as u16;
        if inode_size > 128 {
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }

    pub fn ext4_inode_get_extent_header(&mut self) -> *mut Ext4ExtentHeader {
        let header_ptr = (&mut self.block) as *mut [u32; 15] as *mut Ext4ExtentHeader;
        header_ptr
    }

    pub fn ext4_extent_tree_init(&mut self){
        let mut header = Ext4ExtentHeader::default();
        header.ext4_extent_header_set_depth(0);
        header.ext4_extent_header_set_entries_count(0);
        header.ext4_extent_header_set_generation(0);
        header.ext4_extent_header_set_magic();
        header.ext4_extent_header_set_max_entries_count(4 as u16);

        unsafe {
            let header_ptr = &header as *const Ext4ExtentHeader as *const u32;
            let array_ptr = &mut self.block as *mut [u32; 15] as *mut u32;
            core::ptr::copy_nonoverlapping(header_ptr, array_ptr, 3);
        }
    }

    pub fn ext4_inode_get_blocks_count(&self) -> u64 {
        let mut blocks = self.blocks as u64;
        if self.osd2.l_i_blocks_high != 0 {
            blocks |= (self.osd2.l_i_blocks_high as u64) << 32;
        }
        blocks
    }

    // pub fn ext4_inode_set_blocks_count(&mut self, inode_blocks: u64){
    //     self.blocks = inode_blocks as u32;
    //     self.osd2.l_i_blocks_high = (inode_blocks >> 32) as u16;
    // }
}


// 获取extent header的魔数
#[inline]
pub fn ext4_extent_header_get_magic(header: &Ext4ExtentHeader) -> u16 {
    header.magic
}

// 设置extent header的魔数
#[inline]
pub fn ext4_extent_header_set_magic(header: &mut Ext4ExtentHeader, magic: u16) {
    header.magic = magic;
}

// 获取extent header的条目数
#[inline]
pub fn ext4_extent_header_get_entries_count(header: &Ext4ExtentHeader) -> u16 {
    header.entries_count
}

// 设置extent header的条目数
#[inline]
pub fn ext4_extent_header_set_entries_count(header: &mut Ext4ExtentHeader, count: u16) {
    header.entries_count = count;
}

// 获取extent header的最大条目数
#[inline]
pub fn ext4_extent_header_get_max_entries_count(header: &Ext4ExtentHeader) -> u16 {
    header.max_entries_count
}

// 设置extent header的最大条目数
#[inline]
pub fn ext4_extent_header_set_max_entries_count(header: &mut Ext4ExtentHeader, max_count: u16) {
    header.max_entries_count = max_count;
}

// 获取extent header的深度
#[inline]
pub fn ext4_extent_header_get_depth(header: &Ext4ExtentHeader) -> u16 {
    header.depth
}

// 设置extent header的深度
#[inline]
pub fn ext4_extent_header_set_depth(header: &mut Ext4ExtentHeader, depth: u16) {
    header.depth = depth;
}

// 获取extent header的生成号
#[inline]
pub fn ext4_extent_header_get_generation(header: &Ext4ExtentHeader) -> u32 {
    header.generation
}

// 设置extent header的生成号
#[inline]
pub fn ext4_extent_header_set_generation(header: &mut Ext4ExtentHeader, generation: u32) {
    header.generation = generation;
}

impl Ext4Inode {
    pub fn get_inode_disk_pos(
        &self,
        super_block: &Ext4Superblock,
        block_device: Arc<dyn BlockDevice>,
        inode_id: u32,
    ) -> usize {
        let inodes_per_group = super_block.inodes_per_group;
        let inode_size = super_block.inode_size;
        let group = (inode_id - 1) / inodes_per_group;
        let index = (inode_id - 1) % inodes_per_group;

        let bg = Ext4BlockGroup::load(block_device, super_block, group as usize).unwrap();

        let mut inode_table_blk_num =
            ((bg.inode_table_first_block_hi as u64) << 32) | bg.inode_table_first_block_lo as u64;
        let mut offset =
            inode_table_blk_num as usize * BLOCK_SIZE + (index * inode_size as u32) as usize;
        offset
    }

    pub fn sync_inode_to_disk(
        &self,
        block_device: Arc<dyn BlockDevice>,
        super_block: &Ext4Superblock,
        inode_id: u32,
    ) -> Result<()> {
        let disk_pos = self.get_inode_disk_pos(super_block, block_device.clone(), inode_id);
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Inode>())
        };
        block_device.write_offset(disk_pos, data);

        Ok(())
    }

    pub fn get_inode_checksum(&mut self, inode_id: u32, super_block: &Ext4Superblock) -> u32 {
        let inode_size = super_block.inode_size();

        let orig_checksum = self.get_checksum(super_block);
        let mut checksum = 0;

        let ino_index = inode_id as u32;
        let ino_gen = self.generation;

        // Preparation: temporarily set bg checksum to 0
        self.osd2.l_i_checksum_lo = 0;
        self.i_checksum_hi = 0;

        checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &super_block.uuid,
            super_block.uuid.len() as u32,
        );
        checksum = ext4_crc32c(checksum, &ino_index.to_le_bytes(), 4);
        checksum = ext4_crc32c(checksum, &ino_gen.to_le_bytes(), 4);

        let mut raw_data = [0u8; 0x100];
        copy_inode_to_array(&self, &mut raw_data);

        // inode checksum
        checksum = ext4_crc32c(checksum, &raw_data, inode_size as u32);

        self.set_inode_checksum_value(super_block, inode_id, checksum);

        if inode_size == 128 {
            checksum &= 0xFFFF;
        }

        checksum
    }

    pub fn set_inode_checksum(&mut self, super_block: &Ext4Superblock, inode_id: u32) {
        let inode_size = super_block.inode_size();
        let checksum = self.get_inode_checksum(inode_id, super_block);

        self.osd2.l_i_checksum_lo = ((checksum << 16) >> 16) as u16;
        if inode_size > 128 {
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }

    pub fn sync_inode_to_disk_with_csum(
        &mut self,
        block_device: Arc<dyn BlockDevice>,
        super_block: &Ext4Superblock,
        inode_id: u32,
    ) -> Result<()> {
        self.set_inode_checksum(super_block, inode_id);
        self.sync_inode_to_disk(block_device, super_block, inode_id)
    }
}


pub fn copy_inode_to_array(inode: &Ext4Inode, array: &mut [u8]) {
    unsafe {
        let inode_ptr = inode as *const Ext4Inode as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        core::ptr::copy_nonoverlapping(inode_ptr, array_ptr, 0x9c);
    }
}


#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct Ext4BlockGroup {
    block_bitmap_lo: u32,            // 块位图块
    inode_bitmap_lo: u32,            // 节点位图块
    inode_table_first_block_lo: u32, // 节点表块
    free_blocks_count_lo: u16,       // 空闲块数
    free_inodes_count_lo: u16,       // 空闲节点数
    used_dirs_count_lo: u16,         // 目录数
    flags: u16,                      // EXT4_BG_flags (INODE_UNINIT, etc)
    exclude_bitmap_lo: u32,          // 快照排除位图
    block_bitmap_csum_lo: u16,       // crc32c(s_uuid+grp_num+bbitmap) LE
    inode_bitmap_csum_lo: u16,       // crc32c(s_uuid+grp_num+ibitmap) LE
    itable_unused_lo: u16,           // 未使用的节点数
    checksum: u16,                   // crc16(sb_uuid+group+desc)

    block_bitmap_hi: u32,            // 块位图块 MSB
    inode_bitmap_hi: u32,            // 节点位图块 MSB
    inode_table_first_block_hi: u32, // 节点表块 MSB
    free_blocks_count_hi: u16,       // 空闲块数 MSB
    free_inodes_count_hi: u16,       // 空闲节点数 MSB
    used_dirs_count_hi: u16,         // 目录数 MSB
    itable_unused_hi: u16,           // 未使用的节点数 MSB
    exclude_bitmap_hi: u32,          // 快照排除位图 MSB
    block_bitmap_csum_hi: u16,       // crc32c(s_uuid+grp_num+bbitmap) BE
    inode_bitmap_csum_hi: u16,       // crc32c(s_uuid+grp_num+ibitmap) BE
    reserved: u32,                   // 填充
}

impl TryFrom<&[u8]> for Ext4BlockGroup {
    type Error = u64;
    fn try_from(data: &[u8]) -> core::result::Result<Self, u64> {
        let data = &data[..size_of::<Ext4BlockGroup>()];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl Ext4BlockGroup {

    pub fn get_block_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.block_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.block_bitmap_hi as u64) << 32;
        }
        v
    }

    pub fn get_inode_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.inode_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.inode_bitmap_hi as u64) << 32;
        }
        v
    }

    pub fn get_itable_unused(&mut self, s: &Ext4Superblock) -> u32 {
        let mut v = self.itable_unused_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.itable_unused_hi as u64) << 32) as u32;
        }
        v
    }

    pub fn get_used_dirs_count(&self, s: &Ext4Superblock) -> u32 {
        let mut v = self.used_dirs_count_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.used_dirs_count_hi as u64) << 32) as u32;
        }
        v
    }

    pub fn set_used_dirs_count(&mut self, s: &Ext4Superblock, cnt: u32){
        self.itable_unused_lo = ((cnt << 16) >> 16) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    pub fn set_itable_unused(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = ((cnt << 16) >> 16) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    pub fn set_free_inodes_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.free_inodes_count_lo = ((cnt << 16) >> 16) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.free_inodes_count_hi = (cnt >> 16) as u16;
        }
    }

    pub fn get_free_inodes_count(&self) -> u32 {
        ((self.free_inodes_count_hi as u64) << 32) as u32 | self.free_inodes_count_lo as u32
    }

    pub fn get_inode_table_blk_num(&self) -> u32 {
        ((self.inode_table_first_block_hi as u64) << 32) as u32 | self.inode_table_first_block_lo
    }

    pub fn sync_block_group_to_disk(
        &self,
        block_device: Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        // let dsc_per_block = dsc_cnt;
        let dsc_id = bgid / dsc_cnt;
        // let first_meta_bg = super_block.first_meta_bg;
        let first_data_block = super_block.first_data_block;
        let block_id = first_data_block as usize + dsc_id + 1;
        let offset = (bgid % dsc_cnt) * super_block.desc_size as usize;

        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4BlockGroup>())
        };
        block_device.write_offset(block_id * BLOCK_SIZE + offset, data);
    }

    pub fn get_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) -> u16 {
        let desc_size = super_block.desc_size();

        let mut orig_checksum = 0;
        let mut checksum = 0;

        orig_checksum = self.checksum;

        // 准备：暂时将bg校验和设为0
        self.checksum = 0;

        // uuid checksum
        checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &super_block.uuid,
            super_block.uuid.len() as u32,
        );

        // bgid checksum
        checksum = ext4_crc32c(checksum, &bgid.to_le_bytes(), 4);

        // cast self to &[u8]
        let self_bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 0x40 as usize) };

        // bg checksum
        checksum = ext4_crc32c(checksum, self_bytes, desc_size as u32);

        self.checksum = orig_checksum;

        let crc = (checksum & 0xFFFF) as u16;

        crc
    }

    pub fn set_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) {
        let csum = self.get_block_group_checksum(bgid, super_block);
        self.checksum = csum;
    }

    pub fn sync_to_disk_with_csum(
        &mut self,
        block_device: Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        self.set_block_group_checksum(bgid as u32, super_block);
        self.sync_block_group_to_disk(block_device, bgid, super_block)
    }

    pub fn set_block_group_ialloc_bitmap_csum(&mut self, s: &Ext4Superblock, bitmap: &[u8]) {
        let desc_size = s.desc_size();

        let csum = ext4_ialloc_bitmap_csum(bitmap, s);
        let lo_csum = (csum & 0xFFFF).to_le();
        let hi_csum = (csum >> 16).to_le();

        if (s.features_read_only & 0x400) >> 10 == 0 {
            return;
        }
        self.inode_bitmap_csum_lo = lo_csum as u16;
        if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.inode_bitmap_csum_hi = hi_csum as u16;
        }
    }

    
    pub fn set_block_group_balloc_bitmap_csum(&mut self, s: &Ext4Superblock, bitmap: &[u8]) {
        let desc_size = s.desc_size();

        let csum = ext4_balloc_bitmap_csum(bitmap, s);
        let lo_csum = (csum & 0xFFFF).to_le();
        let hi_csum = (csum >> 16).to_le();

        if (s.features_read_only & 0x400) >> 10 == 0 {
            return;
        }
        self.block_bitmap_csum_lo = lo_csum as u16;
        if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.block_bitmap_csum_hi = hi_csum as u16;
        }
    }

    pub fn get_free_blocks_count(&self) -> u64 {
        let mut v = self.free_blocks_count_lo as u64;
        if self.free_blocks_count_hi != 0 {
            v |= (self.free_blocks_count_hi as u64) << 32;
        }
        v
    }

    pub fn set_free_blocks_count(&mut self, cnt: u64) {
        self.free_blocks_count_lo = ((cnt << 32) >> 32) as u16;
        self.free_blocks_count_hi = (cnt >> 32) as u16;
    }
}

impl Ext4BlockGroup {
    pub fn load(
        block_device: Arc<dyn BlockDevice>,
        super_block: &Ext4Superblock,
        block_group_idx: usize,
        // fs: Weak<Ext4>,
    ) -> core::result::Result<Self, u64> {
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        let dsc_id = block_group_idx / dsc_cnt;
        let first_data_block = super_block.first_data_block;

        let block_id = first_data_block as usize + dsc_id + 1;
        let offset = (block_group_idx % dsc_cnt) * super_block.desc_size as usize;

        let data = block_device.read_offset(block_id * BLOCK_SIZE);

        let block_group_data =
            &data[offset as usize..offset as usize + size_of::<Ext4BlockGroup>()];

        let bg = Ext4BlockGroup::try_from(block_group_data);

        bg
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentHeader {
    /// Magic number, 0xF30A.
    pub magic: u16,

    /// Number of valid entries following the header.
    pub entries_count: u16,

    /// Maximum number of entries that could follow the header.
    pub max_entries_count: u16,

    /// Depth of this extent node in the extent tree.
    /// 0 = this extent node points to data blocks;
    /// otherwise, this extent node points to other extent nodes.
    /// The extent tree can be at most 5 levels deep:
    /// a logical block number can be at most 2^32,
    /// and the smallest n that satisfies 4*(((blocksize - 12)/12)^n) >= 2^32 is 5.
    pub depth: u16,

    /// Generation of the tree. (Used by Lustre, but not standard ext4).
    pub generation: u32,
}

impl<T> TryFrom<&[T]> for Ext4ExtentHeader {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = data;
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl Ext4ExtentHeader{
    pub fn ext4_extent_header_depth(&self) -> u16 {
        self.depth
    }

    pub fn ext4_extent_header_set_depth(&mut self, depth: u16) {
        self.depth = depth;
    }
    pub fn ext4_extent_header_set_entries_count(&mut self, entries_count: u16) {
        self.entries_count = entries_count;
    }
    pub fn ext4_extent_header_set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }
    pub fn ext4_extent_header_set_magic(&mut self) {
        self.magic = EXT4_EXTENT_MAGIC;
    }

    pub fn ext4_extent_header_set_max_entries_count(&mut self, max_entries_count: u16) {
        self.max_entries_count = max_entries_count;
    }

}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentIndex {
    /// This index node covers file blocks from ‘block’ onward.
    pub first_block: u32,

    /// Lower 32-bits of the block number of the extent node that is
    /// the next level lower in the tree. The tree node pointed to
    /// can be either another internal node or a leaf node, described below.
    pub leaf_lo: u32,

    /// Upper 16-bits of the previous field.
    pub leaf_hi: u16,

    pub padding: u16,
}

impl<T> TryFrom<&[T]> for Ext4ExtentIndex {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = &data[..size_of::<Ext4ExtentIndex>()];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
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
    pub block_count: u16,

    /// Upper 16-bits of the block number to which this extent points.
    pub start_hi: u16,

    /// Lower 32-bits of the block number to which this extent points.
    pub start_lo: u32,
}

impl<T> TryFrom<&[T]> for Ext4Extent {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = &data[..size_of::<Ext4Extent>()];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}


/// fake dir entry
#[repr(C)]
pub struct Ext4FakeDirEntry {
    inode: u32,
    entry_length: u16,
    name_length: u8,
    inode_type: u8,
}

// #[derive(Debug)]
// pub struct Ext4ExtentPath {
//     // Physical block number
//     pub p_block: ext4_fsblk_t,
//     // Single block descriptor
//     pub block: Ext4Block,
//     // Depth of this extent node
//     pub depth: u16,
//     // Max depth of the extent tree
//     pub maxdepth: i32,
//     // Pointer to the extent header
//     pub header: *const Ext4ExtentHeader,
//     // Pointer to the index in the current node
//     pub index: *const Ext4ExtentIndex,
//     // Pointer to the extent in the current node
//     pub extent: *const Ext4Extent,
// }

pub fn ext4_first_extent(hdr: *const Ext4ExtentHeader) -> *const Ext4Extent {
    unsafe {
        let offset = core::mem::size_of::<Ext4ExtentHeader>();

        (hdr as *const u8).add(offset) as *const Ext4Extent
    }
}

pub fn ext4_first_extent_mut(hdr: *mut Ext4ExtentHeader) -> *mut Ext4Extent {
    unsafe {
        let offset = core::mem::size_of::<Ext4ExtentHeader>();

        (hdr as *mut u8).add(offset) as *mut Ext4Extent
    }
}

pub fn ext4_last_extent(hdr: *const Ext4ExtentHeader) -> *const Ext4Extent {
    unsafe {
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        let ext_size = core::mem::size_of::<Ext4Extent>();
        let hdr_ref = core::mem::transmute::<*const Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        let ext_count = hdr_ref.entries_count as usize;
        (hdr as *const u8).add(hdr_size + (ext_count - 1) * ext_size) as *const Ext4Extent
    }
}

pub fn ext4_last_extent_mut(hdr: *mut Ext4ExtentHeader) -> *mut Ext4Extent {
    unsafe {
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        let ext_size = core::mem::size_of::<Ext4Extent>();
        let hdr_ref = core::mem::transmute::<*mut Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        let ext_count = hdr_ref.entries_count as usize;

        (hdr as *mut u8).add(hdr_size + (ext_count - 1) * ext_size) as *mut Ext4Extent
    }
}

pub fn ext4_first_extent_index(hdr: *const Ext4ExtentHeader) -> *const Ext4ExtentIndex {
    unsafe {
        let offset = core::mem::size_of::<Ext4ExtentHeader>();

        (hdr as *const u8).add(offset) as *const Ext4ExtentIndex
    }
}

pub fn ext4_first_extent_index_mut(hdr: *mut Ext4ExtentHeader) -> *mut Ext4ExtentIndex {
    unsafe {
        let offset = core::mem::size_of::<Ext4ExtentHeader>();

        (hdr as *mut u8).add(offset) as *mut Ext4ExtentIndex
    }
}

pub fn ext4_last_extent_index(hdr: *const Ext4ExtentHeader) -> *const Ext4ExtentIndex {
    unsafe {
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        let ext_size = core::mem::size_of::<Ext4ExtentIndex>();
        let hdr_ref = core::mem::transmute::<*const Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        let ext_count = hdr_ref.entries_count as usize;
        (hdr as *const u8).add(hdr_size + (ext_count - 1) * ext_size) as *const Ext4ExtentIndex
    }
}

pub fn ext4_last_extent_index_mut(hdr: *mut Ext4ExtentHeader) -> *mut Ext4ExtentIndex {
    unsafe {
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        let ext_size = core::mem::size_of::<Ext4ExtentIndex>();
        let hdr_ref = core::mem::transmute::<*mut Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        let ext_count = hdr_ref.entries_count as usize;
        (hdr as *mut u8).add(hdr_size + (ext_count - 1) * ext_size) as *mut Ext4ExtentIndex
    }
}

pub fn ext4_inode_hdr(inode: &Ext4Inode) -> *const Ext4ExtentHeader {
    let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;
    eh
}

pub fn ext4_inode_hdr_mut(inode: &mut Ext4Inode) -> *mut Ext4ExtentHeader {
    let eh = &mut inode.block as *mut [u32; 15] as *mut Ext4ExtentHeader;
    eh
}

#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentPath {
    // Physical block number
    pub p_block: u64,
    // Single block descriptor
    // pub block: Ext4Block,
    // Depth of this extent node
    pub depth: u16,
    // Max depth of the extent tree
    pub maxdepth: u16,
    // Pointer to the extent header
    pub header: *mut Ext4ExtentHeader,
    // Pointer to the index in the current node
    pub index: *mut Ext4ExtentIndex,
    // Pointer to the extent in the current node
    pub extent: *mut Ext4Extent,
}

impl Default for Ext4ExtentPath {
    fn default() -> Self {
        Self {
            p_block: 0,
            // block: Ext4Block::default(),
            depth: 0,
            maxdepth: 0,
            header: core::ptr::null_mut(),
            index: core::ptr::null_mut(),
            extent: core::ptr::null_mut(),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentPathOld {
    // Physical block number
    pub p_block: u32,
    // Single block descriptor
    // pub block: Ext4Block,
    // Depth of this extent node
    pub depth: u16,
    // Max depth of the extent tree
    pub maxdepth: u16,
    // Pointer to the extent header
    pub header: *const Ext4ExtentHeader,
    // Pointer to the index in the current node
    pub index: *const Ext4ExtentIndex,
    // Pointer to the extent in the current node
    pub extent: *const Ext4Extent,
}

impl Default for Ext4ExtentPathOld {
    fn default() -> Self {
        Self {
            p_block: 0,
            // block: Ext4Block::default(),
            depth: 0,
            maxdepth: 0,
            header: core::ptr::null_mut(),
            index: core::ptr::null_mut(),
            extent: core::ptr::null_mut(),
        }
    }
}
pub struct Ext4InodeRef {
    pub inode_num: u32,
    pub inner: Inner,
    pub fs: Weak<Ext4>,
}

impl Ext4InodeRef {
    pub fn new(fs: Weak<Ext4>) -> Self {
        let mut inner = Inner {
            inode: Ext4Inode::default(),
            weak_self: Weak::new(),
        };

        let mut inode = Self {
            inode_num: 0,
            inner,
            fs,
        };

        inode
    }

    pub fn fs(&self) -> Arc<Ext4> {
        self.fs.upgrade().unwrap()
    }

    pub fn get_inode_ref(fs: Weak<Ext4>, inode_num: u32) -> Self {
        let fs_clone = fs.clone();

        let fs = fs.upgrade().unwrap();
        let super_block = fs.super_block;

        let inodes_per_group = super_block.inodes_per_group;
        let inode_size = super_block.inode_size as u64;
        let group = (inode_num - 1) / inodes_per_group;
        let index = (inode_num - 1) % inodes_per_group;
        let group = fs.block_groups[group as usize];
        let inode_table_blk_num = group.get_inode_table_blk_num();
        let offset =
            inode_table_blk_num as usize * BLOCK_SIZE + index as usize * inode_size as usize;

        let mut data = fs.block_device.read_offset(offset);
        let inode_data = &data[..core::mem::size_of::<Ext4Inode>()];
        let inode = Ext4Inode::try_from(inode_data).unwrap();

        let inner = Inner {
            inode,
            weak_self: Weak::new(),
        };
        let inode = Self {
            inode_num,
            inner,
            fs: fs_clone,
        };

        inode
    }


    pub fn write_back_inode(&mut self) {
        let fs = self.fs();
        let block_device = fs.block_device.clone();
        let super_block = fs.super_block.clone();
        let inode_id = self.inode_num;
        self.inner.inode
            .sync_inode_to_disk_with_csum(block_device, &super_block, inode_id)
            .unwrap()
    }

    pub fn write_back_inode_without_csum(&mut self) {
        let fs = self.fs();
        let block_device = fs.block_device.clone();
        let super_block = fs.super_block.clone();
        let inode_id = self.inode_num;
        self.inner.inode
            .sync_inode_to_disk(block_device, &super_block, inode_id)
            .unwrap()
    }
}

pub struct Inner {
    pub inode: Ext4Inode,
    pub weak_self: Weak<Ext4InodeRef>,
}

impl Inner {
    pub fn inode(&self) -> Arc<Ext4InodeRef> {
        self.weak_self.upgrade().unwrap()
    }

    pub fn write_back_inode(&mut self) {
        let weak_inode_ref = self.weak_self.clone().upgrade().unwrap();
        let fs = weak_inode_ref.fs();
        let block_device = fs.block_device.clone();
        let super_block = fs.super_block.clone();
        let inode_id = weak_inode_ref.inode_num;
        self.inode
            .sync_inode_to_disk_with_csum(block_device, &super_block, inode_id)
            .unwrap()
    }
}

/**@brief   Mount point descriptor.*/
#[derive(Clone)]
pub struct Ext4MountPoint {
    /**@brief   Mount done flag.*/
    pub mounted: bool,
    /**@brief   Mount point name (@ref ext4_mount)*/
    pub mount_name: CString,
    // pub mount_name_string: String,
}
impl Ext4MountPoint {
    pub fn new(name: &str) -> Self {
        Self {
            mounted: false,
            mount_name: CString::new(name).unwrap(),
            // mount_name_string: name.to_string(),
        }
    }
}
impl Debug for Ext4MountPoint {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Ext4MountPoint {{ mount_name: {:?} }}", self.mount_name)
    }
}

#[repr(C)]
pub union Ext4DirEnInternal {
    pub name_length_high: u8, // 高8位的文件名长度
    pub inode_type: u8,       // 引用的inode的类型（在rev >= 0.5中）
}

impl Debug for Ext4DirEnInternal {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        unsafe {
            write!(
                f,
                "Ext4DirEnInternal {{ name_length_high: {:?} }}",
                self.name_length_high
            )
        }
    }
}

impl Default for Ext4DirEnInternal {
    fn default() -> Self {
        Self {
            name_length_high: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Ext4DirEntry {
    pub inode: u32,               // 该目录项指向的inode的编号
    pub entry_len: u16,           // 到下一个目录项的距离
    pub name_len: u8,             // 低8位的文件名长度
    pub inner: Ext4DirEnInternal, // 联合体成员
    pub name: [u8; 255],          // 文件名
}

impl Default for Ext4DirEntry {
    fn default() -> Self {
        Self {
            inode: 0,
            entry_len: 0,
            name_len: 0,
            inner: Ext4DirEnInternal::default(),
            name: [0; 255],
        }
    }
}

impl<T> TryFrom<&[T]> for Ext4DirEntry {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = data;
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

// impl TryFrom<&[u8]> for Ext4DirEntry {
//     type Error = u64;
//     fn try_from(data: &[u8]) -> core::result::Result<Self, u64> {
//         let data = data;
//         Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
//     }
// }

impl Ext4DirEntry{

    pub fn get_name(&self) -> String {
        let name_len = self.name_len as usize;
        let name = &self.name[..name_len];
        let name = core::str::from_utf8(name).unwrap();
        name.to_string()
    }

    pub fn get_name_len(&self) -> usize {
        let name_len = self.name_len as usize;
        name_len
    }

    pub fn ext4_dir_get_csum(&self, s: &Ext4Superblock, blk_data:&[u8]) -> u32{
        
        let ino_index = self.inode;
        let ino_gen = 0 as u32;

        let mut csum = 0;

        let uuid = s.uuid;
    
        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
        csum = ext4_crc32c(csum, &ino_index.to_le_bytes(), 4);
        csum = ext4_crc32c(csum, &ino_gen.to_le_bytes(), 4);
        let mut data = [0u8; 0xff4];
        unsafe{
            core::ptr::copy_nonoverlapping(blk_data.as_ptr(), data.as_mut_ptr(), blk_data.len());
        }
        csum = ext4_crc32c(csum, &data[..], 0xff4);
        csum
    }

    pub fn write_de_to_blk(&self, dst_blk: &mut Ext4Block, offset: usize){
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, count)
        };
        dst_blk.block_data.splice(offset..offset + core::mem::size_of::<Ext4DirEntry>(), data.iter().cloned());
        // assert_eq!(dst_blk.block_data[offset..offset + core::mem::size_of::<Ext4DirEntry>()], data[..]);
    }

}


pub fn copy_dir_entry_to_array(header: &Ext4DirEntry, array: &mut [u8], offset: usize) {
    unsafe {
        let de_ptr = header as *const Ext4DirEntry as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
    }
}

pub fn copy_diren_tail_to_array(dir_en: &Ext4DirEntryTail, array: &mut [u8], offset: usize) {
    unsafe {
        let de_ptr = dir_en as *const Ext4DirEntryTail as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntryTail>();
        core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
    }
}


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntryTail {
    pub reserved_zero1: u32, 
    pub rec_len: u16,        
    pub reserved_zero2: u8,  
    pub reserved_ft: u8,     
    pub checksum: u32,       // crc32c(uuid+inum+dirblock)
}

impl Ext4DirEntryTail{

    pub fn from(data: &mut [u8], blocksize: usize) -> Option<Self> {
        unsafe {
            let ptr = data as *mut [u8] as *mut u8;
            let t = *(ptr.add(blocksize - core::mem::size_of::<Ext4DirEntryTail>()) as *mut Ext4DirEntryTail);
            if t.reserved_zero1 != 0 || t.reserved_zero2 != 0 {
                log::info!("t.reserved_zero1");
                return None;
            }
            if t.rec_len.to_le() != core::mem::size_of::<Ext4DirEntryTail>() as u16 {
                log::info!("t.rec_len");
                return None;
            }
            if t.reserved_ft != 0xDE {
                log::info!("t.reserved_ft");
                return None;
            }
            Some(t)
        }
    }

    pub fn ext4_dir_set_csum(&mut self, s: &Ext4Superblock, diren: &Ext4DirEntry, blk_data: &[u8]){
        let csum = diren.ext4_dir_get_csum(s, blk_data);
        self.checksum = csum;
    }

    pub fn write_de_tail_to_blk(&self, dst_blk: &mut Ext4Block, offset: usize){
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, 0x20)
        };
        dst_blk.block_data.splice(offset..offset + core::mem::size_of::<Ext4DirEntryTail>(), data.iter().cloned());
        assert_eq!(dst_blk.block_data[offset..offset + core::mem::size_of::<Ext4DirEntryTail>()], data[..]);
    }

    pub fn sync_de_tail_to_disk(&self, block_device: Arc<dyn BlockDevice>, dst_blk: &mut Ext4Block){
        let offset = BASE_OFFSET as usize - core::mem::size_of::<Ext4DirEntryTail>();

        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Ext4DirEntryTail>())
        };
        dst_blk.block_data.splice(offset..offset + core::mem::size_of::<Ext4DirEntryTail>(), data.iter().cloned());
        assert_eq!(dst_blk.block_data[offset..offset + core::mem::size_of::<Ext4DirEntryTail>()], data[..]);
        block_device.write_offset(dst_blk.disk_block_id as usize * BLOCK_SIZE, &dst_blk.block_data);
    }



}


pub fn copy_diren_to_array(diren: &Ext4DirEntry, array: &mut [u8]) {
    unsafe {
        let diren_ptr = diren as *const Ext4DirEntry as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        core::ptr::copy_nonoverlapping(diren_ptr, array_ptr, core::mem::size_of::<Ext4DirEntry>());
    }
}

pub struct Ext4DirSearchResult<'a> {
    pub block: Ext4Block<'a>,
    pub dentry: Ext4DirEntry,
}

impl<'a> Ext4DirSearchResult<'a> {
    pub fn new(block: Ext4Block<'a>, dentry: Ext4DirEntry) -> Self {
        Self { block, dentry }
    }
}

#[derive(Debug)]
// A single block descriptor
pub struct Ext4Block<'a> {
    pub logical_block_id: u32, // 逻辑块号

    // disk block id
    pub disk_block_id: u64,

    // size BLOCK_SIZE
    pub block_data: &'a mut Vec<u8>,

    pub dirty: bool,
}

impl <'a>Ext4Block<'a>{
    pub fn sync_blk_to_disk(&self, block_device: Arc<dyn BlockDevice>  ){
        let block_id = self.disk_block_id as usize;
        block_device.write_offset(block_id * BLOCK_SIZE, &self.block_data);
    }
}

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct DirEntryType: u8 {
        const EXT4_DE_UNKNOWN = 0;
        const EXT4_DE_REG_FILE = 1;
        const EXT4_DE_DIR = 2;
        const EXT4_DE_CHRDEV = 3;
        const EXT4_DE_BLKDEV = 4;
        const EXT4_DE_FIFO = 5;
        const EXT4_DE_SOCK = 6;
        const EXT4_DE_SYMLINK = 7;
    }
}

#[derive(Debug, PartialEq)]
pub enum Ext4OpenFlags {
    ReadOnly,
    WriteOnly,
    WriteCreateTrunc,
    WriteCreateAppend,
    ReadWrite,
    ReadWriteCreateTrunc,
    ReadWriteCreateAppend,
}

// 实现一个从字符串转换为标志的函数
// 使用core::str::FromStr特性[^1^][1]
impl core::str::FromStr for Ext4OpenFlags {
    type Err = String;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "r" | "rb" => Ok(Ext4OpenFlags::ReadOnly),
            "w" | "wb" => Ok(Ext4OpenFlags::WriteOnly),
            "a" | "ab" => Ok(Ext4OpenFlags::WriteCreateAppend),
            "r+" | "rb+" | "r+b" => Ok(Ext4OpenFlags::ReadWrite),
            "w+" | "wb+" | "w+b" => Ok(Ext4OpenFlags::ReadWriteCreateTrunc),
            "a+" | "ab+" | "a+b" => Ok(Ext4OpenFlags::ReadWriteCreateAppend),
            _ => Err(crate::ext4_defs::alloc::format!("Unknown open mode: {}", s)),
        }
    }
}

pub fn ext4_ialloc_bitmap_csum(bitmap: &[u8], s: &Ext4Superblock) -> u32 {
    let mut csum = 0;
    let inodes_per_group = s.inodes_per_group;
    let uuid = s.uuid;
    csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
    csum = ext4_crc32c(csum, bitmap, (inodes_per_group + 7) / 8);
    csum
}

pub fn ext4_balloc_bitmap_csum(bitmap: &[u8], s: &Ext4Superblock) -> u32 {
    let mut csum = 0;
    let blocks_per_group = s.blocks_per_group;
    let uuid = s.uuid;
    csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
    csum = ext4_crc32c(csum, bitmap, (blocks_per_group / 8) as u32);
    csum
}


pub fn ext4_ext_binsearch(path: &mut Ext4ExtentPath, block: u32) -> bool {
    // 获取extent header的引用
    // let eh = unsafe { &*path.header };
    let eh = path.header;

    unsafe {
        if (*eh).entries_count == 0 {
            return false;
        }
    }

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent_mut(eh).add(1) };
    let mut r = unsafe { ext4_last_extent_mut(eh) };

    // 如果extent header中没有有效的entry，直接返回false
    unsafe {
        if (*eh).entries_count == 0 {
            return false;
        }
    }
    // 使用while循环进行二分查找
    while l <= r {
        // 计算中间指针
        let m = unsafe { l.add((r as usize - l as usize) / 2) };
        // 获取中间指针所指向的extent的引用
        let ext = unsafe { &*m };
        // 比较逻辑块号和extent的第一个块号
        if block < ext.first_block {
            // 如果逻辑块号小于extent的第一个块号，说明目标在左半边，将右指针移动到中间指针的左边
            r = unsafe { m.sub(1) };
        } else {
            // 如果逻辑块号大于或等于extent的第一个块号，说明目标在右半边，将左指针移动到中间指针的右边
            l = unsafe { m.add(1) };
        }
    }
    // 循环结束后，将path的extent字段设置为左指针的前一个位置
    path.extent = unsafe { l.sub(1) };
    // 返回true，表示找到了对应的extent
    true
}

pub fn ext4_ext_search(path: &mut Ext4ExtentPath, block: u32) -> bool {
    let eh = path.header;

    unsafe {
        if (*eh).entries_count == 0 {
            return false;
        }
    }

    let entries_count = unsafe{(*eh).entries_count};

    let mut extent = unsafe {eh.add(1) as *mut Ext4Extent};

    for i in 0..entries_count{
        let ext = unsafe { &*extent };
        // if block in this ext
        // log::info!("block {:x?} ext.first_block {:x?} last_block {:x?}", block, ext.first_block , (ext.first_block + ext.block_count as u32));
        if block >= ext.first_block && block <= (ext.first_block + ext.block_count as u32) {
            path.extent = extent;
            return true;
        }
        extent = unsafe{extent.add(1)};
    }

    return false;

}

pub fn ext4_ext_binsearch_old(path: &mut Ext4ExtentPathOld, block: u32) -> bool {
    // 获取extent header的引用
    let eh = unsafe { &*path.header };

    if eh.entries_count == 0 {
        false;
    }

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent(eh).add(1) };
    let mut r = unsafe { ext4_last_extent(eh) };

    // 如果extent header中没有有效的entry，直接返回false
    if eh.entries_count == 0 {
        return false;
    }
    // 使用while循环进行二分查找
    while l <= r {
        // 计算中间指针
        let m = unsafe { l.add((r as usize - l as usize) / 2) };
        // 获取中间指针所指向的extent的引用
        let ext = unsafe { &*m };
        // 比较逻辑块号和extent的第一个块号
        if block < ext.first_block {
            // 如果逻辑块号小于extent的第一个块号，说明目标在左半边，将右指针移动到中间指针的左边
            r = unsafe { m.sub(1) };
        } else {
            // 如果逻辑块号大于或等于extent的第一个块号，说明目标在右半边，将左指针移动到中间指针的右边
            l = unsafe { m.add(1) };
        }
    }
    // 循环结束后，将path的extent字段设置为左指针的前一个位置
    path.extent = unsafe { l.sub(1) };
    // 返回true，表示找到了对应的extent
    true
}



pub fn ext4_fs_correspond_inode_mode(filetype: u8) -> u32 {
    // log::info!("filetype: {:?}", filetype);
    let file_type = DirEntryType::from_bits(filetype).unwrap();
    match file_type {
        DirEntryType::EXT4_DE_DIR => EXT4_INODE_MODE_DIRECTORY as u32,
        DirEntryType::EXT4_DE_REG_FILE => EXT4_INODE_MODE_FILE as u32,
        DirEntryType::EXT4_DE_SYMLINK => EXT4_INODE_MODE_SOFTLINK as u32,
        DirEntryType::EXT4_DE_CHRDEV => EXT4_INODE_MODE_CHARDEV as u32,
        DirEntryType::EXT4_DE_BLKDEV => EXT4_INODE_MODE_BLOCKDEV as u32,
        DirEntryType::EXT4_DE_FIFO => EXT4_INODE_MODE_FIFO as u32,
        DirEntryType::EXT4_DE_SOCK => EXT4_INODE_MODE_SOCKET as u32,
        _ => {

            // FIXME: unsupported filetype
            EXT4_INODE_MODE_FILE as u32
        }
    }
}


pub fn ext4_inodes_in_group_cnt(bgid: u32, s: &Ext4Superblock) -> u32 {
    let block_group_count = s.block_groups_count();
    let inodes_per_group = s.inodes_per_group;
    let total_inodes = ((s.inodes_count as u64) << 32) as u32;

    if bgid < block_group_count - 1 {
        inodes_per_group
    } else {
        total_inodes - ((block_group_count - 1) * inodes_per_group)
    }
}


// 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
pub fn ext4_ext_binsearch_idx(path: &mut Ext4ExtentPath, block: ext4_lblk_t) -> bool {
    // 获取extent header的引用
    let eh = path.header;

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent_index_mut(eh).add(1) };
    let mut r = unsafe { ext4_last_extent_index_mut(eh) };

    // 如果extent header中没有有效的entry，直接返回false
    unsafe {
        if (*eh).entries_count == 0 {
            return false;
        }
    }
    // 使用while循环进行二分查找
    while l <= r {
        // 计算中间指针
        let m = unsafe { l.add((r as usize - l as usize) / 2) };
        // 获取中间指针所指向的extent的引用
        let ext = unsafe { &*m };
        // 比较逻辑块号和extent的第一个块号
        if block < ext.first_block {
            // 如果逻辑块号小于extent的第一个块号，说明目标在左半边，将右指针移动到中间指针的左边
            r = unsafe { m.sub(1) };
        } else {
            // 如果逻辑块号大于或等于extent的第一个块号，说明目标在右半边，将左指针移动到中间指针的右边
            l = unsafe { m.add(1) };
        }
    }
    // 循环结束后，将path的extent字段设置为左指针的前一个位置
    path.index = unsafe { l.sub(1) };
    // 返回true，表示找到了对应的extent
    true
}

pub fn ext4_ext_find_extent(
    eh: *mut Ext4ExtentHeader,
    block: ext4_lblk_t,
) -> *mut Ext4Extent {
    // 初始化一些变量
    let mut low: i32;
    let mut high: i32;
    let mut mid: i32;
    let mut ex: *mut Ext4Extent;

    // 如果头部的extent数为0，返回空指针
    if eh.is_null() || unsafe { (*eh).entries_count } == 0 {
        return core::ptr::null_mut();
    }

    // 从头部获取第一个extent的指针
    ex = ext4_first_extent_mut(eh);

    // 如果头部的深度不为0，返回空指针
    if unsafe { (*eh).depth } != 0 {
        return core::ptr::null_mut();
    }

    // 使用二分查找法在extent数组中查找逻辑块号
    low = 0;
    high = unsafe { (*eh).entries_count - 1 } as i32;
    while low <= high {
        // 计算中间位置
        mid = (low + high) / 2;

        // 获取中间位置的extent的指针
        ex = unsafe { ex.add(mid as usize) };

        // 比较extent的逻辑块号和目标逻辑块号
        if block >= unsafe { (*ex).first_block } {
            // 如果目标逻辑块号大于等于extent的逻辑块号，说明目标在右半部分
            low = mid + 1;
        } else {
            // 如果目标逻辑块号小于extent的逻辑块号，说明目标在左半部分
            high = mid - 1;
        }
    }

    // 如果没有找到目标，返回最后一个小于目标的extent的指针
    if high < 0 {
        return core::ptr::null_mut();
    } else {
        return unsafe { ex.add(high as usize) };
    }
}