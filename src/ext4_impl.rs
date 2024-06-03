extern crate alloc;
extern crate log;

use core::mem::size_of;
use core::str;
use core::*;

use crate::consts::*;
use crate::ext4_structs::*;
use crate::prelude::*;
#[allow(unused)]
use crate::return_errno_with_message;
use crate::utils::*;
use crate::BASE_OFFSET;
use crate::BLOCK_SIZE;

// use crate::Ext4Error;

use crate::Ext4;

impl Ext4 {
    pub fn ext4_ialloc_alloc_inode(&self, index: &mut u32, is_dir: bool) {
        log::trace!("ext4_ialloc_alloc_inode");
        let mut bgid = self.last_inode_bg_id;
        let bg_count = self.super_block.block_groups_count();

        while bgid <= bg_count {
            if bgid == bg_count {
                bgid = 0;
                continue;
            }

            let block_device = self.block_device.clone();

            let raw_data = self.block_device.read_offset(BASE_OFFSET);
            let mut super_block = Ext4Superblock::try_from(raw_data).unwrap();

            let mut bg =
                Ext4BlockGroup::load(block_device.clone(), &super_block, bgid as usize).unwrap();

            let mut free_inodes = bg.get_free_inodes_count();
            let mut used_dirs = bg.get_used_dirs_count(&super_block);

            if free_inodes > 0 {
                let inode_bitmap_block = bg.get_inode_bitmap_block(&super_block);

                let mut raw_data = self
                    .block_device
                    .read_offset(inode_bitmap_block as usize * BLOCK_SIZE);

                let inodes_in_bg = super_block.get_inodes_in_group_cnt(bgid);

                // let bitmap_size: u32 = inodes_in_bg / 0x8;

                let mut bitmap_data = &mut raw_data[..];

                let mut idx_in_bg = 0 as u32;

                // log::info!("bitmap {:x?}", bitmap_data);
                ext4_bmap_bit_find_clr(bitmap_data, 0, inodes_in_bg, &mut idx_in_bg);
                ext4_bmap_bit_set(&mut bitmap_data, idx_in_bg);

                // update bitmap in disk
                self.block_device
                    .write_offset(inode_bitmap_block as usize * BLOCK_SIZE, &bitmap_data);

                bg.set_block_group_ialloc_bitmap_csum(&super_block, &bitmap_data);

                /* Modify filesystem counters */
                free_inodes -= 1;
                bg.set_free_inodes_count(&super_block, free_inodes);

                /* Increment used directories counter */
                if is_dir {
                    used_dirs += 1;
                    bg.set_used_dirs_count(&super_block, used_dirs);
                }

                /* Decrease unused inodes count */
                let mut unused = bg.get_itable_unused(&super_block);
                let free = inodes_in_bg - unused as u32;
                if idx_in_bg >= free {
                    unused = inodes_in_bg - (idx_in_bg + 1);
                    bg.set_itable_unused(&super_block, unused);
                }

                bg.sync_to_disk_with_csum(block_device.clone(), bgid as usize, &super_block);
                // bg.sync_block_group_to_disk(block_device.clone(), bgid as usize, &super_block);

                /* Update superblock */
                super_block.decrease_free_inodes_count();
                // super_block.sync_super_block_to_disk(block_device.clone());

                /* Compute the absolute i-nodex number */
                let inodes_per_group = super_block.inodes_per_group();
                let inode_num = bgid * inodes_per_group + (idx_in_bg + 1);
                *index = inode_num;

                // log::info!("alloc inode {:x?}", inode_num);
                return;
            }

            bgid += 1;
        }
        log::info!("no free inode");
    }

    pub fn ext4_fs_put_inode_ref_csum(&self, inode_ref: &mut Ext4InodeRef) {
        inode_ref.write_back_inode();
    }

    pub fn ext4_fs_put_inode_ref(&self, inode_ref: &mut Ext4InodeRef) {
        inode_ref.write_back_inode_without_csum();
    }

    #[allow(unused)]
    pub fn ext4_link(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        name: &str,
        name_len: u32,
    ) -> usize {
        log::trace!(
            "link parent inode {:x?} child inode {:x?} name {:?}",
            parent.inode_num,
            child.inode_num,
            name
        );
        /* Add entry to parent directory */
        let r = self.ext4_dir_add_entry(parent, child, name, name_len);

        /* Fill new dir -> add '.' and '..' entries.
         * Also newly allocated inode should have 0 link count.
         */
        let mut is_dir = false;
        if child.inner.inode.mode & EXT4_INODE_MODE_TYPE_MASK as u16
            == EXT4_INODE_MODE_DIRECTORY as u16
        {
            is_dir = true;
        }

        if is_dir {
            // add '.' and '..' entries
            let fs = child.fs().self_ref.clone();
            let mut child_inode_ref = Ext4InodeRef::new(fs);
            child_inode_ref.inode_num = child.inode_num;
            child_inode_ref.inner.inode = child.inner.inode.clone();

            let r = self.ext4_dir_add_entry(&mut child_inode_ref, child, ".", 1);
            child.inner.inode.size = child_inode_ref.inner.inode.size;
            child.inner.inode.block = child_inode_ref.inner.inode.block;
            let r = self.ext4_dir_add_entry(&mut child_inode_ref, parent, "..", 2);

            child.inner.inode.links_count = 2;
            parent.inner.inode.links_count += 1;

            return EOK;
        }

        child.inner.inode.links_count += 1;
        EOK
    }

    pub fn ext4_dir_add_entry(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        path: &str,
        len: u32,
    ) -> usize {
        let mut iblock = 0;
        let block_size = parent.fs().super_block.block_size();
        let inode_size = parent.inner.inode.inode_get_size();
        // let inode_size = parent.fs().super_block.inode_size_file(&parent.inner.inode);
        let total_blocks = inode_size as u32 / block_size;
        // let mut success = false;

        let mut fblock: Ext4Fsblk = 0;

        log::trace!(
            "ext4_dir_add_entry parent inode {:x?} inode_size {:x?}",
            parent.inode_num,
            inode_size
        );
        while iblock < total_blocks {
            parent.get_inode_dblk_idx(&mut iblock, &mut fblock, false);

            // load_block
            let mut data = parent
                .fs()
                .block_device
                .read_offset(fblock as usize * BLOCK_SIZE);
            let mut ext4_block = Ext4Block {
                logical_block_id: iblock,
                disk_block_id: fblock,
                block_data: &mut data,
                dirty: false,
            };

            let r = self.dir_try_insert_entry(parent, &mut ext4_block, child, path, len);

            if r == EOK {
                return EOK;
            }
            let mut data: Vec<u8> = Vec::with_capacity(BLOCK_SIZE);
            let ext4_blk = Ext4Block {
                logical_block_id: 0,
                disk_block_id: 0,
                block_data: &mut data,
                dirty: true,
            };
            let de = Ext4DirEntry::default();
            let mut dir_search_result = Ext4DirSearchResult::new(ext4_blk, de);

            let r = self.dir_find_in_block(&mut ext4_block, path, len, &mut dir_search_result);

            if r {
                return EOK;
            }

            iblock += 1;
        }

        /* No free block found - needed to allocate next data block */
        iblock = 0;
        fblock = 0;

        // ext4_fs_append_inode_dblk(parent, &mut (iblock as u32), &mut fblock);
        parent.append_inode_dblk(&mut (iblock as u32), &mut fblock);

        /* Load new block */
        let block_device = self.block_device.clone();
        let mut data = block_device.read_offset(fblock as usize * BLOCK_SIZE);
        let mut ext4_block = Ext4Block {
            logical_block_id: iblock,
            disk_block_id: fblock,
            block_data: &mut data,
            dirty: false,
        };

        let mut new_entry = Ext4DirEntry::default();
        let el = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
        self.dir_write_entry(&mut new_entry, el as u16, &child, path, len);
        new_entry.copy_to_slice(&mut ext4_block.block_data, 0);

        copy_dir_entry_to_array(&new_entry, &mut ext4_block.block_data, 0);

        // init tail
        let tail = Ext4DirEntryTail::new();
        tail.copy_to_slice(&mut ext4_block.block_data);

        // set csum
        parent.ext4_dir_set_csum(&mut ext4_block);

        // sync to disk
        ext4_block.sync_blk_to_disk(block_device.clone());
        EOK
    }

    pub fn dir_try_insert_entry(
        &self,
        parent: &Ext4InodeRef,
        dst_blk: &mut Ext4Block,
        child: &mut Ext4InodeRef,
        name: &str,
        name_len: u32,
    ) -> usize {
        log::trace!("dir_try_insert_entry");
        let mut required_len = core::mem::size_of::<Ext4DirEntry>() + name_len as usize;

        if required_len % 4 != 0 {
            required_len += 4 - required_len % 4;
        }

        let mut offset = 0;

        while offset < dst_blk.block_data.len() {
            let mut de = Ext4DirEntry::try_from(&dst_blk.block_data[offset..]).unwrap();
            if de.inode == 0 {
                continue;
            }
            let inode = de.inode;
            let rec_len = de.entry_len;

            // 如果是有效的目录项，尝试分割它
            if inode != 0 {
                let used_len = de.name_len as usize;
                let mut sz = core::mem::size_of::<Ext4FakeDirEntry>() + used_len as usize;

                if used_len % 4 != 0 {
                    sz += 4 - used_len % 4;
                }

                let free_space = rec_len as usize - sz;

                // 如果有足够的空闲空间
                if free_space >= required_len {
                    let mut new_entry = Ext4DirEntry::default();

                    de.entry_len = sz as u16;
                    self.dir_write_entry(&mut new_entry, free_space as u16, &child, name, name_len);

                    // update parent_de and new_de to blk_data
                    de.copy_to_slice(&mut dst_blk.block_data, offset);
                    new_entry.copy_to_slice(&mut dst_blk.block_data, offset + sz);

                    // set tail csum
                    parent.ext4_dir_set_csum(dst_blk);

                    // sync to disk
                    let block_device = self.block_device.clone();
                    dst_blk.sync_blk_to_disk(block_device.clone());

                    return EOK;
                }
            }
            offset = offset + de.entry_len as usize;
        }

        ENOSPC
    }

    // 写入一个ext4目录项
    pub fn dir_write_entry(
        &self,
        en: &mut Ext4DirEntry,
        entry_len: u16,
        child: &Ext4InodeRef,
        name: &str,
        name_len: u32,
    ) {
        let file_type = (child.inner.inode.mode & EXT4_INODE_MODE_TYPE_MASK) as usize;

        // 设置目录项的类型
        match file_type {
            EXT4_INODE_MODE_FILE => en.inner.inode_type = DirEntryType::EXT4_DE_REG_FILE.bits(),
            EXT4_INODE_MODE_DIRECTORY => en.inner.inode_type = DirEntryType::EXT4_DE_DIR.bits(),
            EXT4_INODE_MODE_CHARDEV => en.inner.inode_type = DirEntryType::EXT4_DE_CHRDEV.bits(),
            EXT4_INODE_MODE_BLOCKDEV => en.inner.inode_type = DirEntryType::EXT4_DE_BLKDEV.bits(),
            EXT4_INODE_MODE_FIFO => en.inner.inode_type = DirEntryType::EXT4_DE_FIFO.bits(),
            EXT4_INODE_MODE_SOCKET => en.inner.inode_type = DirEntryType::EXT4_DE_SOCK.bits(),
            EXT4_INODE_MODE_SOFTLINK => en.inner.inode_type = DirEntryType::EXT4_DE_SYMLINK.bits(),
            _ => log::info!("{}: unknown type", file_type),
        }

        en.inode = child.inode_num;
        en.entry_len = entry_len;
        en.name_len = name_len as u8;

        let en_name_ptr = en.name.as_mut_ptr();
        unsafe {
            en_name_ptr.copy_from_nonoverlapping(name.as_ptr(), name_len as usize);
        }
    }

    pub fn ext4_dir_find_entry_new(
        &self,
        parent: &mut Ext4InodeRef,
        name: &str,
    ) -> Result<Ext4DirEntry> {        
        let inode_size: u32 = parent.inner.inode.size;
        let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;
        
        let mut iblock = 0;
        while iblock < total_blocks {
            let fblock = parent.get_pblock(&mut iblock);

            // load_block
            let mut data = parent
                .fs()
                .block_device
                .read_offset(fblock as usize * BLOCK_SIZE);
            let ext4_block = Ext4Block {
                logical_block_id: iblock,
                disk_block_id: fblock,
                block_data: &mut data,
                dirty: false,
            };

            let r = self.dir_find_in_block_new(&ext4_block, name);
            if let Ok(r) = r {
                return Ok(r);
            }
            iblock += 1;
        }
        return_errno_with_message!(Errnum::ENOENT, "file not found");
    }

    pub fn ext4_dir_find_entry(
        &self,
        parent: &mut Ext4InodeRef,
        name: &str,
        name_len: u32,
        result: &mut Ext4DirSearchResult,
    ) -> Result<usize> {
        // log::info!("ext4_dir_find_entry parent {:x?} {:?}",parent.inode_num,  name);
        let mut iblock = 0;
        let mut fblock: Ext4Fsblk = 0;

        let inode_size: u32 = parent.inner.inode.size;
        let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

        while iblock < total_blocks {
            parent.get_inode_dblk_idx(&mut iblock, &mut fblock, false);
            // load_block
            let mut data = parent
                .fs()
                .block_device
                .read_offset(fblock as usize * BLOCK_SIZE);
            let mut ext4_block = Ext4Block {
                logical_block_id: iblock,
                disk_block_id: fblock,
                block_data: &mut data,
                dirty: false,
            };

            let r = self.dir_find_in_block(&mut ext4_block, name, name_len, result);

            result.block_id = fblock as usize;

            if r {
                return Ok(EOK);
            }

            iblock += 1
        }

        return_errno_with_message!(Errnum::ENOENT, "file not found");
    }

    pub fn dir_find_in_block(
        &self,
        block: &Ext4Block,
        name: &str,
        name_len: u32,
        result: &mut Ext4DirSearchResult,
    ) -> bool {
        let mut offset = 0;

        let mut last_de_offset = 0;
        while offset < block.block_data.len() - core::mem::size_of::<Ext4DirEntryTail>() {
            let de = Ext4DirEntry::try_from(&block.block_data[offset..]).unwrap();

            if de.inode == 0 {
                continue;
            }

            let s = get_name(de.name, de.name_len as usize);

            if let Ok(s) = s {
                if name_len == de.name_len as u32 {
                    if name.to_string() == s {
                        result.dentry = de;
                        result.offset = offset;
                        result.last_offset = last_de_offset;
                        return true;
                    }
                }
            }
            last_de_offset = offset;
            offset = offset + de.entry_len as usize;
        }

        false
    }

    pub fn dir_find_in_block_new(
        &self,
        block: &Ext4Block,
        name: &str,
    ) -> Result<Ext4DirEntry> {
        
        let mut offset = 0;
        while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
            let de = Ext4DirEntry::try_from(&block.block_data[offset..]).unwrap();
            if !de.unused() && de.compare_name(name) {
                return Ok(de);
            }
            offset += de.entry_len() as usize;
        }
        return_errno_with_message!(Errnum::ENOENT, "dir find in block failed");
    }

    pub fn dir_find_entry_new(
        &self,
        parent: &mut Ext4InodeRef,
        name: &str,
        name_len: u32,
        result: &mut Ext4DirSearchResult,
    ) -> Result<usize> {
        let mut iblock = 0;
        let mut fblock: Ext4Fsblk = 0;

        let inode_size: u32 = parent.inner.inode.size;
        let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

        while iblock < total_blocks {
            parent.get_inode_dblk_idx(&mut iblock, &mut fblock, false);
            // load_block
            let mut data = parent
                .fs()
                .block_device
                .read_offset(fblock as usize * BLOCK_SIZE);
            let mut ext4_block = Ext4Block {
                logical_block_id: iblock as u32,
                disk_block_id: fblock,
                block_data: &mut data,
                dirty: false,
            };

            let r = self.dir_find_in_block(&mut ext4_block, name, name_len, result);

            result.block_id = fblock as usize;

            if r {
                return Ok(EOK);
            }

            iblock += 1
        }

        return_errno_with_message!(Errnum::ENOENT, "file not found");
    }

    fn ext4_ialloc_get_bgid_of_inode(&self, inode_index: u32) -> u32 {
        inode_index / self.super_block.inodes_per_group()
    }

    fn ext4_ialloc_inode_to_bgidx(&self, inode_index: u32) -> u32 {
        inode_index % self.super_block.inodes_per_group()
    }

    pub fn ext4_ialloc_free_inode(&self, index: u32, is_dir: bool) {
        // Compute index of block group
        let bgid = self.ext4_ialloc_get_bgid_of_inode(index);
        let block_device = self.block_device.clone();
        let raw_data = self.block_device.read_offset(BASE_OFFSET);
        let mut super_block = Ext4Superblock::try_from(raw_data).unwrap();
        let mut bg =
            Ext4BlockGroup::load(block_device.clone(), &super_block, bgid as usize).unwrap();

        // Load inode bitmap block
        let inode_bitmap_block = bg.get_inode_bitmap_block(&self.super_block);
        let mut bitmap_data = self
            .block_device
            .read_offset(inode_bitmap_block as usize * BLOCK_SIZE);

        // Find index within group and clear bit
        let index_in_group = self.ext4_ialloc_inode_to_bgidx(index);
        ext4_bmap_bit_clr(&mut bitmap_data, index_in_group);

        // Set new checksum after modification
        // update bitmap in disk
        self.block_device
            .write_offset(inode_bitmap_block as usize * BLOCK_SIZE, &bitmap_data);
        bg.set_block_group_ialloc_bitmap_csum(&super_block, &bitmap_data);

        // Update free inodes count in block group
        let free_inodes = bg.get_free_inodes_count() + 1;
        bg.set_free_inodes_count(&self.super_block, free_inodes);

        // If inode was a directory, decrement the used directories count
        if is_dir {
            let used_dirs = bg.get_used_dirs_count(&self.super_block) - 1;
            bg.set_used_dirs_count(&self.super_block, used_dirs);
        }

        bg.sync_to_disk_with_csum(block_device.clone(), bgid as usize, &super_block);
        // bg.sync_block_group_to_disk(block_device.clone(), bgid as usize, &super_block);
    }

    #[allow(unused)]
    pub fn ext4_unlink(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        name: &str,
        name_len: u32,
    ) -> usize {
        /* Remove entry from parent directory */
        self.ext4_dir_remove_entry_new(parent, name, name_len);

        self.ext4_ialloc_free_inode(child.inode_num, false);

        EOK
    }
    pub fn ext4_dir_remove_entry_new(&self, parent: &mut Ext4InodeRef, path: &str, len: u32) {
        let mut data: Vec<u8> = Vec::with_capacity(BLOCK_SIZE);
        let ext4_blk = Ext4Block {
            logical_block_id: 0,
            disk_block_id: 0,
            block_data: &mut data,
            dirty: true,
        };
        let mut de = Ext4DirEntry::default();
        let mut dir_search_result = Ext4DirSearchResult::new(ext4_blk, de);

        let r = self.dir_find_entry_new(
            parent,
            &path[..len as usize],
            len as u32,
            &mut dir_search_result,
        );

        dir_search_result.dentry.inode = 0;

        // load_block
        let mut data = parent
            .fs()
            .block_device
            .read_offset(dir_search_result.block_id as usize * BLOCK_SIZE);
        let mut ext4_block = Ext4Block {
            logical_block_id: 0,
            disk_block_id: dir_search_result.block_id as u64,
            block_data: &mut data,
            dirty: false,
        };

        let mut pde =
            Ext4DirEntry::from_u8(&mut ext4_block.block_data[dir_search_result.last_offset..]);

        let mut de_del =
            Ext4DirEntry::from_u8(&mut ext4_block.block_data[dir_search_result.offset..]);

        pde.entry_len += de_del.entry_len;

        let tmp_de_ptr = &pde as *const _ as *const u8;
        let tmp_de_slice = unsafe {
            core::slice::from_raw_parts(tmp_de_ptr, core::mem::size_of::<Ext4DirEntry>())
        };
        ext4_block.block_data[dir_search_result.last_offset
            ..dir_search_result.last_offset + core::mem::size_of::<Ext4DirEntry>()]
            .copy_from_slice(tmp_de_slice);

        parent.ext4_dir_set_csum(&mut ext4_block);
        ext4_block.sync_blk_to_disk(self.block_device.clone());
    }
}
