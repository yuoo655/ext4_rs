use crate::consts::*;
// use crate::BASE_OFFSET;
use super::*;
use crate::prelude::*;
use core::mem::size_of;
// use super::*;
// use crate::consts::*;
// use crate::prelude::*;
use crate::utils::*;
use crate::BlockDevice;
use crate::Ext4;
use crate::BLOCK_SIZE;

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

    pub fn ext4_inode_type(&self, super_block: &Ext4Superblock) -> u32 {
        let mut v = self.mode;

        if super_block.creator_os == EXT4_SUPERBLOCK_OS_HURD {
            v |= ((self.osd2.l_i_file_acl_high as u32) << 16) as u16;
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
        self.size = ((size << 32) >> 32) as u32;
        self.size_hi = (size >> 32) as u32;
    }

    pub fn ext4_inode_get_size(&self) -> u64 {
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

    #[allow(unused)]
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

    pub fn ext4_extent_tree_init(&mut self) {
        let mut header = Ext4ExtentHeader::default();
        header.set_depth(0);
        header.set_entries_count(0);
        header.set_generation(0);
        header.set_magic();
        header.set_max_entries_count(4 as u16);

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

        let inode_table_blk_num =
            ((bg.inode_table_first_block_hi as u64) << 32) | bg.inode_table_first_block_lo as u64;
        let offset =
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

    #[allow(unused)]
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

pub struct Ext4InodeRef {
    pub inode_num: u32,
    pub inner: Inner,
    pub fs: Weak<Ext4>,
}

impl Ext4InodeRef {
    pub fn new(fs: Weak<Ext4>) -> Self {
        let inner = Inner {
            inode: Ext4Inode::default(),
            weak_self: Weak::new(),
        };

        let inode = Self {
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

        let data = fs.block_device.read_offset(offset);
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
        self.inner
            .inode
            .sync_inode_to_disk_with_csum(block_device, &super_block, inode_id)
            .unwrap()
    }

    pub fn write_back_inode_without_csum(&mut self) {
        let fs = self.fs();
        let block_device = fs.block_device.clone();
        let super_block = fs.super_block.clone();
        let inode_id = self.inode_num;
        self.inner
            .inode
            .sync_inode_to_disk(block_device, &super_block, inode_id)
            .unwrap()
    }

    pub fn ext4_fs_put_inode_ref_csum(&mut self) {
        self.write_back_inode();
    }

    pub fn ext4_fs_put_inode_ref(&mut self) {
        self.write_back_inode_without_csum();
    }

    pub fn ext4_fs_inode_blocks_init(&mut self) {
        // log::info!(
        //     "ext4_fs_inode_blocks_init mode {:x?}",
        //     inode_ref.inner.inode.mode
        // );

        let mut inode = self.inner.inode;

        let mode = inode.mode;

        let inode_type = InodeMode::from_bits(mode & EXT4_INODE_MODE_TYPE_MASK as u16).unwrap();

        match inode_type {
            InodeMode::S_IFDIR => {}
            InodeMode::S_IFREG => {}
            /* Reset blocks array. For inode which is not directory or file, just
             * fill in blocks with 0 */
            _ => {
                log::info!("inode_type {:?}", inode_type);
                return;
            }
        }

        /* Initialize extents */
        inode.ext4_inode_set_flags(EXT4_INODE_FLAG_EXTENTS as u32);

        /* Initialize extent root header */
        inode.ext4_extent_tree_init();
        // log::info!("inode iblock {:x?}", inode.block);

        // inode_ref.dirty = true;
    }

    #[allow(unused)]
    pub fn ext4_fs_alloc_inode(&mut self, filetype: u8) -> usize {
        let mut is_dir = false;

        let fs = self.fs();
        let inode_size = self.fs().super_block.inode_size();
        let extra_size = self.fs().super_block.extra_size();

        if filetype == DirEntryType::EXT4_DE_DIR.bits() {
            is_dir = true;
        }

        let mut index = 0;
        let rc = fs.ext4_ialloc_alloc_inode(&mut index, is_dir);

        self.inode_num = index;

        let inode = &mut self.inner.inode;

        /* Initialize i-node */
        let mut mode = 0 as u16;

        if is_dir {
            mode = 0o777;
            mode |= EXT4_INODE_MODE_DIRECTORY as u16;
        } else if filetype == 0x7 {
            mode = 0o777;
            mode |= EXT4_INODE_MODE_SOFTLINK as u16;
        } else {
            mode = 0o666;
            // log::info!("ext4_fs_correspond_inode_mode {:x?}", ext4_fs_correspond_inode_mode(filetype));
            let t = ext4_fs_correspond_inode_mode(filetype);
            mode |= t as u16;
        }

        inode.ext4_inode_set_mode(mode);
        inode.ext4_inode_set_links_cnt(0);
        inode.ext4_inode_set_uid(0);
        inode.ext4_inode_set_gid(0);
        inode.ext4_inode_set_size(0);
        inode.ext4_inode_set_access_time(0);
        inode.ext4_inode_set_change_inode_time(0);
        inode.ext4_inode_set_modif_time(0);
        inode.ext4_inode_set_del_time(0);
        inode.ext4_inode_set_flags(0);
        inode.ext4_inode_set_generation(0);

        if inode_size > EXT4_GOOD_OLD_INODE_SIZE {
            let extra_size = extra_size;
            inode.ext4_inode_set_extra_isize(extra_size);
        }

        EOK
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

/// Ext4 inode-related operations.
impl Ext4Inode {
    /// Get a pointer to the extent header from an inode.
    pub fn extent_header(&self) -> *const Ext4ExtentHeader {
        &self.block as *const [u32; 15] as *const Ext4ExtentHeader
    }

    /// Get a mutable pointer to the extent header from an inode.
    pub fn extent_header_mut(&mut self) -> *mut Ext4ExtentHeader {
        &mut self.block as *mut [u32; 15] as *mut Ext4ExtentHeader
    }

    /// Get the depth of the extent tree from an inode.
    pub unsafe fn extent_depth(&self) -> u16 {
        (*self.extent_header()).depth
    }
}
