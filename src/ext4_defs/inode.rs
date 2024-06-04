use crate::prelude::*;
use crate::utils::*;

use super::*;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Inode {
    pub mode: u16,           // 文件类型和权限
    pub uid: u16,            // 所有者用户 ID
    pub size: u32,           // 低 32 位文件大小
    pub atime: u32,          // 最近访问时间
    pub ctime: u32,          // 创建时间
    pub mtime: u32,          // 最近修改时间
    pub dtime: u32,          // 删除时间
    pub gid: u16,            // 所有者组 ID
    pub links_count: u16,    // 链接计数
    pub blocks: u32,         // 已分配的块数
    pub flags: u32,          // 文件标志
    pub osd1: u32,           // 操作系统相关的字段1
    pub block: [u32; 15],    // 数据块指针
    pub generation: u32,     // 文件版本（NFS）
    pub file_acl: u32,       // 文件 ACL
    pub size_hi: u32,        // 高 32 位文件大小
    pub faddr: u32,          // 已废弃的碎片地址
    pub osd2: Linux2,        // 操作系统相关的字段2

    pub i_extra_isize: u16,    // 额外的 inode 大小
    pub i_checksum_hi: u16,    // 高位校验和（crc32c(uuid+inum+inode) BE）
    pub i_ctime_extra: u32,    // 额外的创建时间（纳秒 << 2 | 纪元）
    pub i_mtime_extra: u32,    // 额外的修改时间（纳秒 << 2 | 纪元）
    pub i_atime_extra: u32,    // 额外的访问时间（纳秒 << 2 | 纪元）
    pub i_crtime: u32,         // 创建时间
    pub i_crtime_extra: u32,   // 额外的创建时间（纳秒 << 2 | 纪元）
    pub i_version_hi: u32,     // 高 32 位版本
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Linux2 {
    pub l_i_blocks_high: u16,    // 高 16 位已分配块数
    pub l_i_file_acl_high: u16,  // 高 16 位文件 ACL
    pub l_i_uid_high: u16,       // 高 16 位用户 ID
    pub l_i_gid_high: u16,       // 高 16 位组 ID
    pub l_i_checksum_lo: u16,    // 低位校验和
    pub l_i_reserved: u16,       // 保留字段
}


bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct InodeFileType: u16 {
        const S_IFIFO = 0x1000;
        const S_IFCHR = 0x2000;
        const S_IFDIR = 0x4000;
        const S_IFBLK = 0x6000;
        const S_IFREG = 0x8000;
        const S_IFSOCK = 0xC000;
        const S_IFLNK = 0xA000;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct InodeAttr: u16 {
        const S_IREAD = 0x0100;
        const S_IWRITE = 0x0080;
        const S_IEXEC = 0x0040;
        const S_ISUID = 0x0800;
        const S_ISGID = 0x0400;
    }
}

impl Ext4Inode {
    pub fn mode(&self) -> u16 {
        self.mode
    }

    pub fn set_mode(&mut self, mode: u16) {
        self.mode = mode;
    }

    pub fn uid(&self) -> u16 {
        self.uid
    }

    pub fn set_uid(&mut self, uid: u16) {
        self.uid = uid;
    }

    pub fn size(&self) -> u64 {
        self.size as u64 | ((self.size_hi as u64) << 32)
    }

    pub fn set_size(&mut self, size: u64) {
        self.size = ((size << 32) >> 32) as u32;
        self.size_hi = (size >> 32) as u32;
    }

    pub fn atime(&self) -> u32 {
        self.atime
    }

    pub fn set_atime(&mut self, atime: u32) {
        self.atime = atime;
    }

    pub fn ctime(&self) -> u32 {
        self.ctime
    }

    pub fn set_ctime(&mut self, ctime: u32) {
        self.ctime = ctime;
    }

    pub fn mtime(&self) -> u32 {
        self.mtime
    }

    pub fn set_mtime(&mut self, mtime: u32) {
        self.mtime = mtime;
    }

    pub fn dtime(&self) -> u32 {
        self.dtime
    }

    pub fn set_dtime(&mut self, dtime: u32) {
        self.dtime = dtime;
    }

    pub fn gid(&self) -> u16 {
        self.gid
    }

    pub fn set_gid(&mut self, gid: u16) {
        self.gid = gid;
    }

    pub fn links_count(&self) -> u16 {
        self.links_count
    }

    pub fn set_links_count(&mut self, links_count: u16) {
        self.links_count = links_count;
    }

    pub fn blocks(&self) -> u64 {
        let mut blocks = self.blocks as u64;
        if self.osd2.l_i_blocks_high != 0 {
            blocks |= (self.osd2.l_i_blocks_high as u64) << 32;
        }
        blocks
    }

    pub fn set_blocks(&mut self, blocks: u64) {
        self.blocks = (blocks & 0xFFFFFFFF) as u32;
        self.osd2.l_i_blocks_high = (blocks >> 32) as u16;
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }

    pub fn osd1(&self) -> u32 {
        self.osd1
    }

    pub fn set_osd1(&mut self, osd1: u32) {
        self.osd1 = osd1;
    }

    pub fn block(&self) -> [u32; 15] {
        self.block
    }

    pub fn set_block(&mut self, block: [u32; 15]) {
        self.block = block;
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    pub fn file_acl(&self) -> u32 {
        self.file_acl
    }

    pub fn set_file_acl(&mut self, file_acl: u32) {
        self.file_acl = file_acl;
    }

    pub fn size_hi(&self) -> u32 {
        self.size_hi
    }

    pub fn set_size_hi(&mut self, size_hi: u32) {
        self.size_hi = size_hi;
    }

    pub fn faddr(&self) -> u32 {
        self.faddr
    }

    pub fn set_faddr(&mut self, faddr: u32) {
        self.faddr = faddr;
    }

    pub fn osd2(&self) -> Linux2 {
        self.osd2
    }

    pub fn set_osd2(&mut self, osd2: Linux2) {
        self.osd2 = osd2;
    }

    pub fn i_extra_isize(&self) -> u16 {
        self.i_extra_isize
    }

    pub fn set_i_extra_isize(&mut self, i_extra_isize: u16) {
        self.i_extra_isize = i_extra_isize;
    }

    pub fn i_checksum_hi(&self) -> u16 {
        self.i_checksum_hi
    }

    pub fn set_i_checksum_hi(&mut self, i_checksum_hi: u16) {
        self.i_checksum_hi = i_checksum_hi;
    }

    pub fn i_ctime_extra(&self) -> u32 {
        self.i_ctime_extra
    }

    pub fn set_i_ctime_extra(&mut self, i_ctime_extra: u32) {
        self.i_ctime_extra = i_ctime_extra;
    }

    pub fn i_mtime_extra(&self) -> u32 {
        self.i_mtime_extra
    }

    pub fn set_i_mtime_extra(&mut self, i_mtime_extra: u32) {
        self.i_mtime_extra = i_mtime_extra;
    }

    pub fn i_atime_extra(&self) -> u32 {
        self.i_atime_extra
    }

    pub fn set_i_atime_extra(&mut self, i_atime_extra: u32) {
        self.i_atime_extra = i_atime_extra;
    }

    pub fn i_crtime(&self) -> u32 {
        self.i_crtime
    }

    pub fn set_i_crtime(&mut self, i_crtime: u32) {
        self.i_crtime = i_crtime;
    }

    pub fn i_crtime_extra(&self) -> u32 {
        self.i_crtime_extra
    }

    pub fn set_i_crtime_extra(&mut self, i_crtime_extra: u32) {
        self.i_crtime_extra = i_crtime_extra;
    }

    pub fn i_version_hi(&self) -> u32 {
        self.i_version_hi
    }

    pub fn set_i_version_hi(&mut self, i_version_hi: u32) {
        self.i_version_hi = i_version_hi;
    }
}





impl Ext4Inode{
    pub fn file_type(&self) -> InodeFileType {
        InodeFileType::from_bits_truncate(self.mode & 0xF000)
    }

    pub fn inode_attr(&self) -> InodeAttr {
        InodeAttr::from_bits_truncate(self.mode & 0x0FFF)
    }

    pub fn is_dir(&self) -> bool {
        self.file_type() == InodeFileType::S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        self.file_type() == InodeFileType::S_IFREG
    }

    pub fn is_link(&self) -> bool {
        self.file_type() == InodeFileType::S_IFLNK
    }

    pub fn can_read(&self) -> bool {
        self.inode_attr().contains(InodeAttr::S_IREAD)
    }

    pub fn can_write(&self) -> bool {
        self.inode_attr().contains(InodeAttr::S_IWRITE)
    }

    pub fn can_exec(&self) -> bool {
        self.inode_attr().contains(InodeAttr::S_IEXEC)
    }


}


/// Reference to an inode.
pub struct Ext4InodeRef {
    pub inode_num: u32,
    pub inode: Ext4Inode,
}
