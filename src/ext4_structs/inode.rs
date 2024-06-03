use super::*;
use crate::consts::*;
use crate::prelude::*;
#[allow(unused)]
use crate::return_errno_with_message;
use crate::utils::*;
use crate::BlockDevice;
use crate::Ext4;
use crate::BLOCK_SIZE;
use core::mem::size_of;

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

    pub fn ext4_inode_get_links_cnt(&self) -> u16 {
        self.links_count
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

    pub fn inode_get_size(&self) -> u64 {
        self.size as u64 | ((self.size_hi as u64) << 32)
    }

    pub fn ext4_inode_set_atime(&mut self, access_time: u32) {
        self.atime = access_time;
    }

    pub fn ext4_inode_get_atime(&self) -> u32 {
        self.atime
    }

    pub fn ext4_inode_set_ctime(&mut self, change_inode_time: u32) {
        self.ctime = change_inode_time;
    }

    pub fn ext4_inode_get_ctime(&self) -> u32 {
        self.ctime
    }

    pub fn ext4_inode_set_mtime(&mut self, modif_time: u32) {
        self.mtime = modif_time;
    }

    pub fn ext4_inode_get_mtime(&self) -> u32 {
        self.mtime
    }

    pub fn ext4_inode_set_del_time(&mut self, del_time: u32) {
        self.dtime = del_time;
    }

    pub fn ext4_inode_set_crtime(&mut self, crtime: u32) {
        self.i_crtime = crtime;
    }
    pub fn ext4_inode_get_crtime(&self) -> u32 {
        self.i_crtime
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

    fn copy_to_slice(&self, slice: &mut [u8]) {
        unsafe {
            let inode_ptr = self as *const Ext4Inode as *const u8;
            let array_ptr = slice.as_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(inode_ptr, array_ptr, 0x9c);
        }
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
        self.copy_to_slice(&mut raw_data);

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

    /// Get a pointer to the extent header from an inode.
    pub fn extent_header_new(&self) -> &Ext4ExtentHeader {
        unsafe {
            (&self.block as *const [u32; 15] as *const Ext4ExtentHeader)
                .as_ref()
                .unwrap()
        }
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

impl Ext4InodeRef {
    #[allow(unused)]
    /// Searches for an extent and initializes path data structures accordingly.
    pub fn find_extent(
        &mut self,
        block_id: Ext4Lblk,
        orig_path: &mut Option<Vec<Ext4ExtentPath>>,
        flags: u32,
    ) -> usize {
        let inode = &self.inner.inode;
        let mut eh: &Ext4ExtentHeader;
        let mut buf_block: Ext4Fsblk = 0;
        let mut path = orig_path.take(); // Take the path out of the Option, which may replace it with None
        let depth = unsafe { self.inner.inode.extent_depth() };

        let mut ppos = 0;
        let mut i: u16;

        let eh = &inode.block as *const [u32; 15] as *mut Ext4ExtentHeader;

        if let Some(ref mut p) = path {
            if depth > p[0].maxdepth {
                p.clear();
            }
        }
        if path.is_none() {
            let path_depth = depth + 1;
            path = Some(vec![Ext4ExtentPath::default(); path_depth as usize + 1]);
            path.as_mut().unwrap()[0].maxdepth = path_depth;
        }

        let path = path.as_mut().unwrap();
        path[0].header = eh;

        i = depth;
        while i > 0 {
            path[ppos].binsearch_extentidx(block_id);
            // ext4_ext_binsearch_idx(&mut path[ppos], block);
            path[ppos].p_block = unsafe { *path[ppos].index }.pblock();
            path[ppos].depth = i;
            path[ppos].extent = core::ptr::null_mut();
            buf_block = path[ppos].p_block;

            i -= 1;
            ppos += 1;
        }

        path[ppos].depth = i;
        path[ppos].extent = core::ptr::null_mut();
        path[ppos].index = core::ptr::null_mut();

        path[ppos].search_extent(block_id);
        if !path[ppos].extent.is_null() {
            path[ppos].p_block = unsafe { (*path[ppos].extent).pblock() } as u64;
        }

        *orig_path = Some(path.clone());

        EOK
    }

    #[allow(unused)]
    /// Gets blocks from an inode and manages extent creation if necessary.
    pub fn get_blocks(
        &mut self,
        iblock: Ext4Lblk,
        max_blocks: u32,
        result: &mut Ext4Fsblk,
        create: bool,
        blocks_count: &mut u32,
    ) {
        *result = 0;
        *blocks_count = 0;

        let mut path: Option<Vec<Ext4ExtentPath>> = None;
        // let err = crate::ext4_find_extent(self, iblock, &mut path, 0);
        let err = self.find_extent(iblock, &mut path, 0);

        // Ensure find_extent was successful
        if err != EOK {
            return;
        }
        // Unwrap the path safely after checking it is Some
        let path = path
            .as_mut()
            .expect("Path should not be None after successful find_extent");

        let depth = unsafe { self.inner.inode.extent_depth() } as usize;
        // Safely access the desired depth element if it exists
        if let Some(extent_path) = path.get(depth) {
            if let Some(ex) = unsafe { extent_path.extent.as_ref() } {
                // Safely obtain a reference to extent
                let ee_block = ex.first_block;
                let ee_start = ex.pblock();
                let ee_len = ex.get_actual_len();

                if iblock >= ee_block && iblock < ee_block + ee_len as u32 {
                    let allocated = ee_len - (iblock - ee_block) as u16;
                    *blocks_count = allocated as u32;

                    if !create || ex.is_unwritten() {
                        *result = (iblock - ee_block + ee_start) as u64;
                        return; // Early return if no new extent needed
                    }
                }
            }
        }
        if create {
            let mut allocated: u32 = 0;
            let next = EXT_MAX_BLOCKS;

            allocated = next - iblock;
            if allocated > max_blocks {
                allocated = max_blocks;
            }

            let mut newex: Ext4Extent = Ext4Extent::default();

            let goal = 0;

            let mut alloc_block = 0;
            alloc_block = self.balloc_alloc_block(goal as u64);

            *result = alloc_block;

            // 创建并插入新的extent
            newex.first_block = iblock;
            newex.start_lo = alloc_block as u32 & 0xffffffff;
            newex.start_hi = (((alloc_block as u32) << 31) << 1) as u16;
            newex.block_count = allocated as u16;

            // crate::ext4_ext_insert_extent(self,  &mut path[0], &newex, 0);

            self.insert_extent(&mut path[0], &newex, 0);
            // self.allocate_and_insert_new_extent(iblock, max_blocks, result, blocks_count);
        }
    }

    #[allow(unused)]
    pub fn balloc_alloc_block(&mut self, goal: Ext4Fsblk) -> u64 {
        log::trace!("balloc_alloc_block");
        // let mut fblock = 0;

        let fs = self.fs();

        let block_device = fs.block_device.clone();

        let super_block_data = block_device.read_offset(crate::BASE_OFFSET);
        let mut super_block = Ext4Superblock::try_from(super_block_data).unwrap();

        // let inodes_per_group = super_block.inodes_per_group();
        let blocks_per_group = super_block.blocks_per_group();

        let bgid = goal / blocks_per_group as u64;
        let idx_in_bg = goal % blocks_per_group as u64;

        let mut bg =
            Ext4BlockGroup::load(block_device.clone(), &super_block, bgid as usize).unwrap();

        let block_bitmap_block = bg.get_block_bitmap_block(&super_block);
        let mut raw_data = block_device.read_offset(block_bitmap_block as usize * BLOCK_SIZE);
        let mut data: &mut Vec<u8> = &mut raw_data;
        let mut rel_blk_idx = 0 as u32;
        // let blk_in_bg = bg.ext4_blocks_in_group_cnt(&super_block);

        ext4_bmap_bit_find_clr(data, idx_in_bg as u32, 0x8000, &mut rel_blk_idx);
        // fblock = rel_blk_idx as u64;
        ext4_bmap_bit_set(&mut data, rel_blk_idx);

        bg.set_block_group_balloc_bitmap_csum(&super_block, &data);
        block_device.write_offset(block_bitmap_block as usize * BLOCK_SIZE, &data);

        /* Update superblock free blocks count */
        let mut super_blk_free_blocks = super_block.free_blocks_count();
        super_blk_free_blocks -= 1;
        super_block.set_free_blocks_count(super_blk_free_blocks);
        super_block.sync_to_disk_with_csum(block_device.clone());

        /* Update inode blocks (different block size!) count */
        let mut inode_blocks = self.inner.inode.ext4_inode_get_blocks_count();
        inode_blocks += 8;
        self.inner
            .inode
            .ext4_inode_set_blocks_count(inode_blocks as u32);
        self.write_back_inode();

        /* Update block group free blocks count */
        let mut fb_cnt = bg.get_free_blocks_count();
        fb_cnt -= 1;
        bg.set_free_blocks_count(fb_cnt as u32);
        bg.sync_to_disk_with_csum(block_device, bgid as usize, &super_block);

        rel_blk_idx as u64
    }

    /// Inserts a new extent into the inode's data structure.
    pub fn insert_extent(&mut self, path: &mut Ext4ExtentPath, newext: &Ext4Extent, flags: i32) {
        let depth = unsafe { self.inner.inode.extent_depth() };
        let mut need_split = false;

        self.insert_leaf(path, depth, newext, flags, &mut need_split);

        self.write_back_inode_without_csum();
    }

    #[allow(unused)]
    /// Handles the leaf insertion logic for extents, considering append and prepend scenarios.
    fn insert_leaf(
        &mut self,
        path: &mut Ext4ExtentPath,
        _depth: u16,
        newext: &Ext4Extent,
        _flags: i32,
        need_split: &mut bool,
    ) -> usize {
        // Ensure we are working with a valid extent header and existing extent
        if let Some(eh) = unsafe { path.header.as_mut() } {
            let ex = path.extent;

            // Manage appending new extents or adjusting existing ones
            if let Some(current_ext) = unsafe { ex.as_mut() } {
                // Calculate disk block for the new extent
                let diskblock = newext.pblock();

                // Append new extent to existing one if possible
                if current_ext.can_append(newext) {
                    if current_ext.is_unwritten() {
                        unsafe {
                            Ext4Extent::mark_unwritten(current_ext);
                        }
                    }

                    // Update the block count to include the new extent
                    current_ext.block_count += newext.get_actual_len();
                    path.p_block = diskblock as u64;
                    return EOK; // Successful append
                }

                // Prepend new extent to existing one if possible
                if current_ext.can_prepend(newext) {
                    current_ext.first_block = newext.first_block;
                    current_ext.block_count += newext.get_actual_len();
                    path.p_block = diskblock as u64;

                    if current_ext.is_unwritten() {
                        unsafe {
                            Ext4Extent::mark_unwritten(current_ext);
                        }
                    }
                    return EOK; // Successful prepend
                }
            }

            // If no existing extent to append or prepend, we need to insert a new extent
            unsafe {
                if eh.entries_count == eh.max_entries_count {
                    *need_split = true;
                    return EIO; // Indicate error due to max entries reached
                }

                // Insert the new extent if there is space
                let first_extent = Ext4ExtentHeader::first_extent_mut(eh);
                if ex.is_null() {
                    path.extent = first_extent;
                    *first_extent = *newext;
                } else {
                    // Handling inserting after the current extent
                    let next_extent = ex.add(1);
                    path.extent = next_extent;
                    *next_extent = *newext;
                }

                eh.entries_count += 1;
            }
        }

        EOK // Indicate successful operation
    }

    #[allow(unused)]
    pub fn get_inode_dblk_idx(
        &mut self,
        iblock: &mut Ext4Lblk,
        fblock: &mut Ext4Fsblk,
        extent_create: bool,
    ) -> usize {
        let current_block: Ext4Fsblk;
        let mut current_fsblk: Ext4Fsblk = 0;

        let mut blocks_count = 0;
        // crate::ext4_extent_get_blocks(self,*iblock, 1, &mut current_fsblk, false, &mut blocks_count);
        self.get_blocks_new(*iblock, 1, &mut current_fsblk, false, &mut blocks_count);

        current_block = current_fsblk;
        *fblock = current_block;

        EOK
    }

    #[allow(unused)]
    pub fn get_pblock(&mut self, iblock: &mut Ext4Lblk) -> Ext4Fsblk {
        let current_block: Ext4Fsblk;
        let mut current_fsblk: Ext4Fsblk = 0;

        let mut blocks_count = 0;

        self.get_blocks_new(*iblock, 1, &mut current_fsblk, false, &mut blocks_count);

        current_fsblk
    }

    #[allow(unused)]
    pub fn ext4_fs_get_inode_dblk_idx_internal(
        &mut self,
        iblock: &mut Ext4Lblk,
        fblock: &mut Ext4Fsblk,
        extent_create: bool,
        support_unwritten: bool,
    ) {
        let mut current_block: Ext4Fsblk;
        let mut current_fsblk: Ext4Fsblk = 0;

        let mut blocks_count = 0;
        // crate::ext4_extent_get_blocks(self,*iblock, 1, &mut current_fsblk, false, &mut blocks_count);
        self.get_blocks(
            *iblock,
            1,
            &mut current_fsblk,
            extent_create,
            &mut blocks_count,
        );
    }

    pub fn append_inode_dblk(&mut self, iblock: &mut Ext4Lblk, fblock: &mut Ext4Fsblk) {
        let inode_size = self.inner.inode.inode_get_size();
        let block_size = BLOCK_SIZE as u64;

        *iblock = ((inode_size + block_size - 1) / block_size) as u32;

        let current_block: Ext4Fsblk;
        let mut current_fsblk: Ext4Fsblk = 0;
        // ext4_extent_get_blocks(inode_ref, *iblock, 1, &mut current_fsblk, true, &mut 0);
        self.get_blocks(*iblock, 1, &mut current_fsblk, true, &mut 0);

        current_block = current_fsblk;
        *fblock = current_block;

        self.inner
            .inode
            .ext4_inode_set_size(inode_size + BLOCK_SIZE as u64);

        self.write_back_inode();

        // let mut inode_ref = Ext4InodeRef::get_inode_ref(inode_ref.fs().self_ref.clone(), inode_ref.inode_num);

        // log::info!("ext4_fs_append_inode_dblk inode {:x?} inode_size {:x?}", inode_ref.inode_num, inode_ref.inner.inode.size);
        // log::info!("fblock {:x?}", fblock);
    }

    pub fn ext4_fs_inode_blocks_init(&mut self) {
        // log::info!(
        //     "ext4_fs_inode_blocks_init mode {:x?}",
        //     inode_ref.inner.inode.mode
        // );

        let inode = &mut self.inner.inode;

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

        let inode_size = self.fs().super_block.inode_size();
        let extra_size = self.fs().super_block.extra_size();

        if filetype == DirEntryType::EXT4_DE_DIR.bits() {
            is_dir = true;
        }

        let mut index = 0;
        let rc = self.fs().ext4_ialloc_alloc_inode(&mut index, is_dir);

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
        inode.ext4_inode_set_atime(0);
        inode.ext4_inode_set_ctime(0);
        inode.ext4_inode_set_mtime(0);
        inode.ext4_inode_set_del_time(0);
        inode.ext4_inode_set_flags(0);
        inode.ext4_inode_set_generation(0);

        if inode_size > EXT4_GOOD_OLD_INODE_SIZE {
            let extra_size = extra_size;
            inode.ext4_inode_set_extra_isize(extra_size);
        }

        EOK
    }

    pub fn ext4_find_all_extent(&self, extents: &mut Vec<Ext4Extent>) {
        let extent_header = Ext4ExtentHeader::try_from(&self.inner.inode.block[..2]).unwrap();
        // log::info!("extent_header {:x?}", extent_header);
        let data = &self.inner.inode.block;
        let depth = extent_header.depth;

        self.ext4_add_extent(depth, data, extents, true);
    }

    #[allow(unused)]
    pub fn ext4_add_extent(
        &self,
        depth: u16,
        data: &[u32],
        extents: &mut Vec<Ext4Extent>,
        first_level: bool,
    ) {
        let extent_header = Ext4ExtentHeader::try_from(data).unwrap();
        let extent_entries = extent_header.entries_count;
        // log::info!("extent_entries {:x?}", extent_entries);
        if depth == 0 {
            for en in 0..extent_entries {
                let idx = (3 + en * 3) as usize;
                let extent = Ext4Extent::try_from(&data[idx..]).unwrap();

                extents.push(extent)
            }
            return;
        }

        for en in 0..extent_entries {
            let idx = (3 + en * 3) as usize;
            if idx == 12 {
                break;
            }
            let extent_index = Ext4ExtentIndex::try_from(&data[idx..]).unwrap();
            let ei_leaf_lo = extent_index.leaf_lo;
            let ei_leaf_hi = extent_index.leaf_hi;
            let mut block = ei_leaf_lo;
            block |= ((ei_leaf_hi as u32) << 31) << 1;
            let data = self
                .fs()
                .block_device
                .read_offset(block as usize * BLOCK_SIZE);
            let data: Vec<u32> = unsafe { core::mem::transmute(data) };
            self.ext4_add_extent(depth - 1, &data, extents, false);
        }
    }
}

impl Ext4InodeRef {
    #[allow(unused)]
    /// Searches for an extent and initializes path data structures accordingly.
    pub fn find_extent_new(&self, block_id: Ext4Lblk) -> Vec<Ext4ExtentPathNew> {
        let mut path: Vec<Ext4ExtentPathNew> = Vec::new();

        let root_data = &self.inner.inode.block[..];
        let root_tree = ExtentTreeNode::load_from_header(&root_data[..]);

        let fs = self.fs();
        let block_device = fs.block_device.clone();

        root_tree.find_extent(block_id, block_device, &mut path);

        path
    }

    #[allow(unused)]

    /// Gets blocks from an inode and manages extent creation if necessary.
    pub fn get_blocks_new(
        &mut self,
        iblock: Ext4Lblk,
        max_blocks: u32,
        result: &mut Ext4Fsblk,
        create: bool,
        blocks_count: &mut u32,
    ) {
        *result = 0;
        *blocks_count = 0;

        let mut path: Vec<Ext4ExtentPathNew> = self.find_extent_new(iblock);

        let last = path.last().unwrap();

        if let Some(pblock) = last.p_block {
            let ee_start = pblock as u32;
            let ee_block = last.first_block as u32;
            let ee_len = last.block_count as u32;

            if iblock >= ee_block && iblock < ee_block + ee_len as u32 {
                let allocated = ee_len - (iblock - ee_block) as u32;
                *blocks_count = allocated as u32;
            }

            let ex = Ext4Extent {
                first_block: ee_block,
                block_count: ee_len as u16,
                start_hi: last.start_hi,
                start_lo: last.start_lo,
            };
            if !create || ex.is_unwritten() {
                *result = (iblock - ee_block + ee_start) as u64;
                return; // Early return if no new extent needed
            }
        }
        if create {
            let mut allocated: u32 = 0;
            let next = EXT_MAX_BLOCKS;

            allocated = next - iblock;
            if allocated > max_blocks {
                allocated = max_blocks;
            }

            let mut newex: Ext4Extent = Ext4Extent::default();

            let goal = 0;

            let mut alloc_block = 0;
            alloc_block = self.balloc_alloc_block(goal as u64);

            *result = alloc_block;

            // 创建并插入新的extent
            newex.first_block = iblock;
            newex.start_lo = alloc_block as u32 & 0xffffffff;
            newex.start_hi = (((alloc_block as u32) << 31) << 1) as u16;
            newex.block_count = allocated as u16;
        }
    }

    /// Gets blocks from an inode and manages extent creation if necessary.
    pub fn find_extent_foo(&self, iblock: Ext4Lblk) -> Vec<Ext4ExtentPathNew> {
        let path: Vec<Ext4ExtentPathNew> = self.find_extent_new(iblock);

        path
    }
}

impl Ext4InodeRef {
    pub fn ext4_dir_set_csum(&self, dst_blk: &mut Ext4Block) {
        let parent_de = Ext4DirEntry::try_from(&dst_blk.block_data[..]).unwrap();
        let mut tail = Ext4DirEntryTail::from(&mut dst_blk.block_data, BLOCK_SIZE).unwrap();

        let ino_gen = self.inner.inode.generation;
        tail.ext4_dir_set_csum(
            &self.fs().super_block,
            &parent_de,
            &dst_blk.block_data[..],
            ino_gen,
        );

        tail.copy_to_slice(&mut dst_blk.block_data);
    }
}

impl Ext4InodeRef {
    pub fn truncate_inode(&mut self, new_size: u64) -> Result<usize> {
        let new_size = new_size as usize;

        let old_size = self.inner.inode.inode_get_size() as usize;

        if old_size == new_size {
            return Ok(EOK);
        }

        let block_size = BLOCK_SIZE;
        let new_blocks_cnt = ((new_size + block_size - 1) / block_size) as u32;
        let old_blocks_cnt = ((old_size + block_size - 1) / block_size) as u32;
        let diff_blocks_cnt = old_blocks_cnt - new_blocks_cnt;

        if diff_blocks_cnt > 0 {
            let r = self.extent_remove_space(new_blocks_cnt, EXT_MAX_BLOCKS as u32);
        }

        self.inner.inode.ext4_inode_set_size(new_size as u64);
        self.write_back_inode();

        // return_errno_with_message!(Errnum::ENOTSUP, "not support");
        return Ok(EOK);
    }

    pub fn extent_remove_space(&mut self, from: u32, to: u32) -> Result<usize> {
        let depth = unsafe { self.inner.inode.extent_depth() } as usize;
        let mut path: Option<Vec<Ext4ExtentPath>> = None;
        let err = self.find_extent(from.into(), &mut path, 0);

        if err != EOK {
            return_errno_with_message!(
                Errnum::ENOTSUP,
                "extent_remove_space ext4_find_extent fail"
            );
        }

        let path = path
            .as_mut()
            .expect("Path should not be None after successful find_extent");
        let mut extent = unsafe { *path[depth].extent };

        let in_range = (from..=to).contains(&(extent.first_block.into()));

        if !in_range {
            return Ok(EOK);
        }

        // remove_space inside the range of this extent
        if (extent.first_block < from)
            && (to < (extent.first_block + extent.block_count as u32 - 1))
        {
            let mut newex: Ext4Extent = Ext4Extent::default();
            let unwritten = extent.is_unwritten();
            let ee_block = extent.first_block;
            let block_count = extent.block_count;

            let newblock = to + 1 - ee_block + extent.pblock();

            extent.block_count = from as u16 - ee_block as u16;

            if unwritten {
                extent.mark_unwritten();
            }

            newex.first_block = to + 1;
            newex.block_count = (ee_block + block_count as u32 - 1 - to) as u16;
            newex.start_lo = newblock as u32 & 0xffffffff;
            newex.start_hi = (((newblock as u32) << 31) << 1) as u16;

            self.insert_extent(&mut path[0], &newex, 0);
        }

        if depth == 0 {
            let header = Ext4ExtentHeader::try_from(&self.inner.inode.block[..]).unwrap();

            let first_ex = Ext4Extent::try_from(&self.inner.inode.block[3..]).unwrap();
            let entry_count = header.entries_count as usize;
            let idx = entry_count * 3 as usize;
            let last_ex = Ext4Extent::try_from(&self.inner.inode.block[idx..]).unwrap();

            let mut leaf_from = first_ex.first_block;
            let mut leaf_to = last_ex.first_block + last_ex.get_actual_len() as u32 - 1;

            if leaf_from < from {
                leaf_from = from;
            }
            if leaf_to > to {
                leaf_to = to;
            }
            let r = self.ext_remove_leaf(path, leaf_from, leaf_to);

            // let node =  ExtentTreeNode::load_from_header(&self.inner.inode.block[..]);
        }

        let header = Ext4ExtentHeader::try_from(&self.inner.inode.block[..]).unwrap();

        if header.entries_count == 0 {
            let mut header = Ext4ExtentHeader::try_from_u32(&mut self.inner.inode.block[..]);
            self.inner.inode.block[3..].fill(0);
            self.write_back_inode();
        }
        Ok(EOK)
    }

    // from 20 to 29

    // 初始状态:
    // +------------------+------------------+------------------+
    // | extent 0 (10-14)   | extent 1 (20-29)   | extent 2 (35-42)   |
    // +------------------+------------------+------------------+

    // extent 0 (10-14): (不在范围)
    // +------------------+------------------+------------------+
    // | extent 0 (10-14)   | extent 1 (20-29)   | extent 2 (35-42)   |
    // +------------------+------------------+------------------+

    // 处理extent 1 (20-29):（在范围, 删除）
    // +------------------+------------------+------------------+
    // | extent 0 (10-14)   | 删除 (空)        | extent 2 (35-42)   |
    // +------------------+------------------+------------------+

    // 移动剩余extent:
    // +------------------+------------------+------------------+
    // | extent 0 (10-14)   | extent 2 (35-42)   | (空)             |
    // +------------------+------------------+------------------+

    // 最终结果:
    // +------------------+------------------+------------------+
    // | extent 0 (10-14)   | extent 1 (35-42)   | (空)             |
    // +------------------+------------------+------------------+

    pub fn ext_remove_leaf(
        &mut self,
        path: &mut Vec<Ext4ExtentPath>,
        from: u32,
        to: u32,
    ) -> Result<usize> {
        let depth = unsafe { self.inner.inode.extent_depth() } as usize;

        let mut header = unsafe { *path[depth].header };

        // let mut first = Ext4Extent::try_from(&self.inner.inode.block[3..]).unwrap();

        // let mut first_ex = &mut first as *mut Ext4Extent;

        // (first as *mut Ext4Extent as *mut u8) as *mut Ext4Extent

        let mut new_entry_count = header.entries_count;

        let entry_count = header.entries_count;
        let start_ex = 0;

        let mut ex2: Ext4Extent = Ext4Extent::default();

        for i in 0..entry_count as usize {
            let idx = 3 + i * 3;
            let mut ex = Ext4Extent::try_from(&self.inner.inode.block[idx..]).unwrap();

            if ex.first_block > to {
                break;
            }

            let mut new_len = 0;
            let mut start = ex.first_block;
            let mut new_start = ex.first_block;

            let mut len = ex.get_actual_len();
            let mut newblock = ex.pblock();

            if start < from {
                len -= from as u16 - start as u16;
                new_len = from - start;
                start = from;
            } else {
                if start + len as u32 - 1 > to {
                    new_len = start + len as u32 - 1 - to;
                    len -= new_len as u16;

                    new_start = to + 1;

                    newblock += to + 1 - start;

                    ex2 = ex;
                }
            }

            self.ext_remove_blocks(&mut ex, start, start + len as u32 - 1);

            ex.first_block = new_start;

            if new_len == 0 {
                new_entry_count -= 1;
            } else {
                let unwritten = ex.is_unwritten();
                ex.store_pblock(newblock as u64);
                ex.block_count = new_len as u16;

                if unwritten {
                    ex.mark_unwritten();
                }
            }
        }

        // Move any remaining extents to the starting position of the node.
        if ex2.first_block > 0 {
            let start_index = 3 + start_ex * 3;
            let end_index = 3 + entry_count as usize * 3;
            let remaining_extents: Vec<u32> =
                self.inner.inode.block[start_index..end_index].to_vec();
            self.inner.inode.block[3..3 + remaining_extents.len()]
                .copy_from_slice(&remaining_extents);
        }

        // Update the entries count in the header
        header.entries_count = new_entry_count;

        // Fix the indexes if the extent pointer is at the first extent
        if start_ex == 0 && new_entry_count > 0 {
            self.ext_correct_indexes(path)?;
        }

        /* if this leaf is free, then we should
         * remove it from index block above */

        // fixme leaf is root should not be removed
        if new_entry_count == 0 {
            let mut header = Ext4ExtentHeader::try_from_u32(&mut self.inner.inode.block[..]);
            header.entries_count = 0;
            unsafe {
                let header_ptr = &header as *const Ext4ExtentHeader as *const u32;
                let array_ptr = &mut self.inner.inode.block as *mut [u32; 15] as *mut u32;
                core::ptr::copy_nonoverlapping(header_ptr, array_ptr, 3);
            }
        }

        Ok(EOK)
    }

    fn ext_correct_indexes(&mut self, path: &mut Vec<Ext4ExtentPath>) -> Result<usize> {
        let depth = unsafe { self.inner.inode.extent_depth() } as usize;

        let mut ex = path[depth].extent;
        let mut header = path[depth].header;

        if ex.is_null() || header.is_null() {
            return_errno_with_message!(Errnum::EIO, "no header");
        }

        // let mut header = unsafe { *path[depth].header };

        if depth == 0 {
            // there is no tree at all
            return Ok(EOK);
        }

        if ex != unsafe { (*path[depth].header).first_extent_mut() } {
            // we correct tree if first leaf got modified only
            return Ok(EOK);
        }

        let mut k = depth - 1;
        let border = unsafe { *ex }.first_block;
        unsafe { *path[k].index }.first_block = border;

        while k > 0 {
            // change all left-side indexes
            if path[k + 1].index != unsafe { (*path[k + 1].header).first_extent_index_mut() } {
                break;
            }
            unsafe { *path[k].index }.first_block = border;
            k -= 1;
        }

        Ok(EOK)
    }

    fn ext_remove_idx(&mut self, path: &mut Vec<Ext4ExtentPath>, depth: i32) -> Result<usize> {
        Ok(EOK)
    }

    pub fn ext_remove_blocks(&mut self, ex: &mut Ext4Extent, from: u32, to: u32) {
        let len = to - from + 1;

        let num = from - ex.first_block;

        let start = ex.pblock() + num;

        self.balloc_free_blocks(start as _, len);
    }

    #[allow(unused)]
    pub fn balloc_free_blocks(&mut self, start: Ext4Fsblk, count: u32) {
        let mut count = count as usize;
        let mut start = start;

        let fs = self.fs();

        let block_device = fs.block_device.clone();

        let super_block_data = block_device.read_offset(crate::BASE_OFFSET);
        let mut super_block = Ext4Superblock::try_from(super_block_data).unwrap();

        // let inodes_per_group = super_block.inodes_per_group();
        let blocks_per_group = super_block.blocks_per_group();

        let bgid = start / blocks_per_group as u64;

        let mut bg_first = start / blocks_per_group as u64;
        let mut bg_last = (start + count as u64 - 1) / blocks_per_group as u64;

        while bg_first <= bg_last {
            let idx_in_bg = start % blocks_per_group as u64;

            let mut bg =
                Ext4BlockGroup::load(block_device.clone(), &super_block, bg_first as usize)
                    .unwrap();

            let block_bitmap_block = bg.get_block_bitmap_block(&super_block);
            let mut raw_data = block_device.read_offset(block_bitmap_block as usize * BLOCK_SIZE);
            let mut data: &mut Vec<u8> = &mut raw_data;

            let mut free_cnt = BLOCK_SIZE * 8 - idx_in_bg as usize;

            if count as usize > free_cnt {
            } else {
                free_cnt = count as usize;
            }

            ext4_bmap_bits_free(&mut data, idx_in_bg as u32, free_cnt as u32);

            count -= free_cnt;
            start += free_cnt as u64;

            bg.set_block_group_balloc_bitmap_csum(&super_block, &data);
            block_device.write_offset(block_bitmap_block as usize * BLOCK_SIZE, &data);

            /* Update superblock free blocks count */
            let mut super_blk_free_blocks = super_block.free_blocks_count();

            // fixme
            super_blk_free_blocks += free_cnt as u64;
            super_block.set_free_blocks_count(super_blk_free_blocks);
            super_block.sync_to_disk_with_csum(block_device.clone());

            let super_block_data = block_device.read_offset(crate::BASE_OFFSET);
            let mut super_block = Ext4Superblock::try_from(super_block_data).unwrap();

            /* Update inode blocks (different block size!) count */
            let mut inode_blocks = self.inner.inode.ext4_inode_get_blocks_count();

            inode_blocks -= (free_cnt as usize * (BLOCK_SIZE / EXT4_INODE_BLOCK_SIZE)) as u64;
            self.inner
                .inode
                .ext4_inode_set_blocks_count(inode_blocks as u32);
            self.write_back_inode();

            /* Update block group free blocks count */
            let mut fb_cnt = bg.get_free_blocks_count();
            fb_cnt += free_cnt as u64;
            bg.set_free_blocks_count(fb_cnt as u32);
            bg.sync_to_disk_with_csum(block_device.clone(), bgid as usize, &super_block);

            bg_first += 1;
        }
    }

    pub fn ext4_dir_get_csum(&self, s: &Ext4Superblock, blk_data: &[u8]) -> u32 {
        let ino_index = self.inode_num;
        let ino_gen = 0 as u32;

        let mut csum = 0;

        let uuid = s.uuid;

        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
        csum = ext4_crc32c(csum, &ino_index.to_le_bytes(), 4);
        csum = ext4_crc32c(csum, &ino_gen.to_le_bytes(), 4);
        let mut data = [0u8; 0xff4];
        unsafe {
            core::ptr::copy_nonoverlapping(blk_data.as_ptr(), data.as_mut_ptr(), blk_data.len());
        }
        csum = ext4_crc32c(csum, &data[..], 0xff4);
        csum
    }

    // pub fn set_inode_mode(&mut self, mode: u16) {
    //     self.inner.inode.mode = mode;
    // }

    // pub fn set_inode_uid(&mut self, uid: u16) {
    //     self.inner.inode.ext4_inode_set_uid(uid)
    // }

    // pub fn set_inode_gid(&mut self, gid: u16) {
    //     self.inner.inode.ext4_inode_set_gid(gid)
    // }

    // pub fn set_inode_size(&mut self, size: u64) {
    //     self.inner.inode.ext4_inode_set_size(size)
    // }

    // pub fn set_inode_atime(&mut self, atime: u32) {
    //     self.inner.inode.ext4_inode_set_atime(atime)
    // }

    // pub fn set_inode_mtime(&mut self, mtime: u32) {
    //     self.inner.inode.ext4_inode_set_ctime(mtime)
    // }

    // pub fn set_inode_ctime(&mut self, ctime: u32) {
    //     self.inner.inode.ext4_inode_set_ctime(ctime)
    // }

    // pub fn set_inode_crtime(&mut self, crtime: u32) {
    //     self.inner.inode.ext4_inode_set_crtime(crtime)
    // }

    // pub fn set_inode_chgtime(&mut self, mode: u16) {
    //     self.inner.inode.mode = mode;
    // }

    // pub fn set_inode_bkuptime(&mut self, mode: u16) {
    //     self.inner.inode.mode = mode;
    // }

    // pub fn set_inode_flags(&mut self, mode: u16) {
    //     self.inner.inode.mode = mode;
    // }
}

impl Ext4InodeRef {
    pub fn inode_is_type(&self, inode_type: u32) -> bool {
        let super_block = self.fs().super_block;
        self.inner.inode.ext4_inode_type(&super_block) == inode_type
    }

    pub fn is_dir(&self) -> bool {
        self.inode_is_type(EXT4_INODE_MODE_DIRECTORY as u32)
    }

    // find a entry
    pub fn inode_has_entry(&self) -> bool {
        let inode_ref = self;

        let mut iblock = 0;
        let block_size = inode_ref.fs().super_block.block_size();
        let inode_size = inode_ref.inner.inode.inode_get_size();
        let total_blocks = inode_size as u32 / block_size;
        let mut fblock: Ext4Fsblk = 0;

        while iblock < total_blocks {
            let path: Vec<Ext4ExtentPathNew> = self.find_extent_foo(iblock);

            if path.len() == 0 {
                return false;
            }
            let last = path.last().unwrap();
            if let Some(pblock) = last.p_block {
                let ee_start = pblock as u32;
                let ee_block = last.first_block as u32;
                fblock = (iblock - ee_block + ee_start) as u64;
            }

            // load_block
            let mut data = inode_ref
                .fs()
                .block_device
                .read_offset(fblock as usize * BLOCK_SIZE);
            let ext4_block = Ext4Block {
                logical_block_id: iblock,
                disk_block_id: fblock,
                block_data: &mut data,
                dirty: false,
            };

            let mut offset = 0;
            while offset < ext4_block.block_data.len() {
                let de = Ext4DirEntry::try_from(&ext4_block.block_data[offset..]).unwrap();
                offset = offset + de.entry_len as usize;
                if de.inode == 0 {
                    continue;
                }
                
                // skip . and ..
                if de.get_name() == "." || de.get_name() == ".." {
                    continue;
                }
                return true;
            }
            iblock += 1
        }

        false
    }

    pub fn has_children(&self) -> bool {
        if !self.is_dir() {
            return false;
        }
        self.inode_has_entry()
    }
}
