extern crate alloc;
extern crate log;

use core::str;
use core::*;

use crate::consts::*;
use crate::ext4_structs::*;
use crate::prelude::*;
use crate::return_errno_with_message;
use crate::utils::*;
use crate::Ext4Error;

pub trait Jbd2: Send + Sync + Any + Debug {
    fn load_journal(&mut self);
    fn journal_start(&mut self);
    fn transaction_start(&mut self);
    fn write_transaction(&mut self, block_id: usize, block_data: Vec<u8>);
    fn transaction_stop(&mut self);
    fn journal_stop(&mut self);
    fn recover(&mut self);
}

pub trait BlockDevice: Send + Sync + Any {
    fn read_offset(&self, offset: usize) -> Vec<u8>;
    fn write_offset(&self, offset: usize, data: &[u8]);
}

// impl dyn BlockDevice {
//     pub fn downcast_ref<T: BlockDevice>(&self) -> Option<&T> {
//         (self as &dyn Any).downcast_ref::<T>()
//     }
// }

pub struct Ext4 {
    pub block_device: Arc<dyn BlockDevice>,
    pub super_block: Ext4Superblock,
    pub block_groups: Vec<Ext4BlockGroup>,
    pub inodes_per_group: u32,
    pub blocks_per_group: u32,
    pub inode_size: usize,
    pub last_inode_bg_id: u32,
    pub self_ref: Weak<Self>,
    pub mount_point: Ext4MountPoint,
}

/// ext4 对外接口
impl Ext4 {
    #[allow(unused)]
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Self> {
        // Load the superblock
        // TODO: if the main superblock is corrupted, should we load the backup?
        let raw_data = block_device.read_offset(BASE_OFFSET);
        let super_block = Ext4Superblock::try_from(raw_data).unwrap();

        // log::info!("super_block: {:x?}", super_block);
        let inodes_per_group = super_block.inodes_per_group();
        let blocks_per_group = super_block.blocks_per_group();
        let inode_size = super_block.inode_size();

        // Load the block groups information
        let load_block_groups =
            |fs: Weak<Ext4>, block_device: Arc<dyn BlockDevice>| -> Result<Vec<Ext4BlockGroup>> {
                let block_groups_count = super_block.block_groups_count() as usize;
                let mut block_groups = Vec::with_capacity(block_groups_count);
                for idx in 0..block_groups_count {
                    let block_group =
                        Ext4BlockGroup::load(block_device.clone(), &super_block, idx).unwrap();
                    block_groups.push(block_group);
                }
                Ok(block_groups)
            };

        let mount_point = Ext4MountPoint::new("/");

        let ext4: Arc<Ext4> = Arc::new_cyclic(|weak_ref| Self {
            super_block: super_block,
            inodes_per_group: inodes_per_group,
            blocks_per_group: blocks_per_group,
            inode_size: inode_size as usize,
            block_groups: load_block_groups(weak_ref.clone(), block_device.clone()).unwrap(),
            block_device,
            self_ref: weak_ref.clone(),
            mount_point: mount_point,
            last_inode_bg_id: 0,
        });

        ext4
    }

    // 使用libc库定义的常量
    fn ext4_parse_flags(&self, flags: &str) -> Result<u32> {
        match flags {
            "r" | "rb" => Ok(O_RDONLY),
            "w" | "wb" => Ok(O_WRONLY | O_CREAT | O_TRUNC),
            "a" | "ab" => Ok(O_WRONLY | O_CREAT | O_APPEND),
            "r+" | "rb+" | "r+b" => Ok(O_RDWR),
            "w+" | "wb+" | "w+b" => Ok(O_RDWR | O_CREAT | O_TRUNC),
            "a+" | "ab+" | "a+b" => Ok(O_RDWR | O_CREAT | O_APPEND),
            _ => Err(Ext4Error::new(Errnum::EINVAL)),
        }
    }

    // start transaction
    pub fn ext4_trans_start(&self) {}

    // stop transaction
    pub fn ext4_trans_abort(&self) {}

    pub fn update_super_block(&mut self) {
        let raw_data = self.block_device.read_offset(BASE_OFFSET);
        let super_block = Ext4Superblock::try_from(raw_data).unwrap();
        self.super_block = super_block;
    }

    #[allow(unused)]
    pub fn ext4_open(
        &self,
        file: &mut Ext4File,
        path: &str,
        flags: &str,
        file_expect: bool,
    ) -> Result<usize> {
        let mut iflags = 0;
        let mut filetype = DirEntryType::EXT4_DE_UNKNOWN;

        // get mount point
        // let mut ptr = Box::new(self.mount_point.clone());
        file.mp = self.mount_point.clone();

        // get open flags
        iflags = self.ext4_parse_flags(flags).unwrap();

        // file for dir
        if file_expect {
            filetype = DirEntryType::EXT4_DE_REG_FILE;
        } else {
            filetype = DirEntryType::EXT4_DE_DIR;
        }

        if iflags & O_CREAT != 0 {
            self.ext4_trans_start();
        }

        let mut root_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);

        let r = self.ext4_generic_open(file, path, iflags, filetype.bits(), &mut root_inode_ref);

        r
    }

    pub fn ext4_file_close(&self, file: &mut Ext4File) -> Result<usize> {
        // assert!(!file.mp.is_null());

        file.mp = self.mount_point.clone();
        file.flags = 0;
        file.inode = 0;
        file.fpos = 0;
        file.fsize = 0;

        return Ok(EOK);
    }
    #[allow(unused)]
    pub fn ext4_dir_mk(&self, path: &str) -> Result<usize> {
        log::trace!("ext4_dir_mk {:?}", path);
        let mut file = Ext4File::new();
        let flags = "w";

        let mut iflags = 0;
        let filetype = DirEntryType::EXT4_DE_DIR;

        // get mount point
        // let mut ptr = Box::new(self.mount_point.clone());
        file.mp = self.mount_point.clone();

        // get open flags
        iflags = self.ext4_parse_flags(flags).unwrap();

        if iflags & O_CREAT != 0 {
            self.ext4_trans_start();
        }

        let mut root_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);

        let r = self.ext4_generic_open(
            &mut file,
            path,
            iflags,
            filetype.bits(),
            &mut root_inode_ref,
        );

        // log::info!("dir mk done");
        r
    }

    #[allow(unused)]
    pub fn ext4_generic_open(
        &self,
        file: &mut Ext4File,
        path: &str,
        iflags: u32,
        ftype: u8,
        parent_inode: &mut Ext4InodeRef,
    ) -> Result<usize> {
        let mut is_goal = false;

        let mp: &Ext4MountPoint = &self.mount_point;

        let mp_name = mp.mount_name.as_bytes();

        let mut data: Vec<u8> = Vec::with_capacity(BLOCK_SIZE);
        let ext4_blk = Ext4Block {
            logical_block_id: 0,
            disk_block_id: 0,
            block_data: &mut data,
            dirty: true,
        };
        let mut de = Ext4DirEntry::default();
        let mut dir_search_result = Ext4DirSearchResult::new(ext4_blk, de);

        file.flags = iflags;

        // load root inode
        let mut root_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);

        if path == "" {
            // open root directory
            file.inode = 2;
            file.fpos = 0;
            file.fsize = root_inode_ref.inner.inode.inode_get_size();
            return Ok(EOK);
        }
        // if !parent_inode.is_none() {
        //     parent_inode.unwrap().inode_num = root_inode_ref.inode_num;
        // }

        // search dir
        let mut search_parent = root_inode_ref;
        let mut search_path = ext4_path_skip(&path, ".");
        let mut len = 0;
        loop {
            search_path = ext4_path_skip(search_path, "/");
            len = ext4_path_check(search_path, &mut is_goal);

            let r = self.ext4_dir_find_entry(
                &mut search_parent,
                &search_path[..len as usize],
                len as u32,
                &mut dir_search_result,
            );

            // log::info!("dir_search_result.dentry {:?} r {:?}", dir_search_result.dentry, r);
            if let Err(e) = r {
                if e.error() != Errnum::ENOENT.into() || (iflags & O_CREAT) == 0 {
                    return_errno_with_message!(Errnum::ENOENT, "file not found and not create");
                }

                let mut child_inode_ref = Ext4InodeRef::new(self.self_ref.clone());

                let mut r;

                if is_goal {
                    r = child_inode_ref.ext4_fs_alloc_inode(ftype);
                    // r = ext4_fs_alloc_inode(&mut child_inode_ref, ftype);
                } else {
                    r = child_inode_ref.ext4_fs_alloc_inode(DirEntryType::EXT4_DE_DIR.bits());
                    // r = ext4_fs_alloc_inode(&mut child_inode_ref, DirEntryType::EXT4_DE_DIR.bits())
                }

                if r != EOK {
                    return_errno_with_message!(Errnum::EALLOCFIAL, "alloc inode fail");
                    // break;
                }

                child_inode_ref.ext4_fs_inode_blocks_init();
                // ext4_fs_inode_blocks_init(&mut child_inode_ref);

                let r = self.ext4_link(
                    &mut search_parent,
                    &mut child_inode_ref,
                    &search_path[..len as usize],
                    len as u32,
                );

                if r != EOK {
                    /*Fail. Free new inode.*/
                    return_errno_with_message!(Errnum::ELINKFIAL, "link fail");
                }

                self.ext4_fs_put_inode_ref_csum(&mut search_parent);
                self.ext4_fs_put_inode_ref_csum(&mut child_inode_ref);
                self.ext4_fs_put_inode_ref_csum(parent_inode);

                continue;
            }

            let name = get_name(
                dir_search_result.dentry.name,
                dir_search_result.dentry.name_len as usize,
            )
            .unwrap();
            // log::info!("find de name{:?} de inode {:x?}", name, dir_search_result.dentry.inode);

            if is_goal {
                let current_inode_ref = Ext4InodeRef::get_inode_ref(
                    self.self_ref.clone(),
                    dir_search_result.dentry.inode,
                );
                file.inode = dir_search_result.dentry.inode;
                file.fpos = 0;
                file.fsize = current_inode_ref.inner.inode.inode_get_size();

                return Ok(EOK);
            } else {
                search_parent = Ext4InodeRef::get_inode_ref(
                    self.self_ref.clone(),
                    dir_search_result.dentry.inode,
                );
                search_path = &search_path[len..];
            }
        }
    }

    // with dir search path offset
    pub fn ext4_generic_open2(
        &self,
        file: &mut Ext4File,
        path: &str,
        iflags: u32,
        ftype: u8,
        parent_inode: &mut Ext4InodeRef,
        name_off: &mut u32,
    ) -> Result<usize> {
        let mut is_goal = false;

        let mut data: Vec<u8> = Vec::with_capacity(BLOCK_SIZE);
        let ext4_blk = Ext4Block {
            logical_block_id: 0,
            disk_block_id: 0,
            block_data: &mut data,
            dirty: true,
        };
        let de = Ext4DirEntry::default();
        let mut dir_search_result = Ext4DirSearchResult::new(ext4_blk, de);

        // Load the root inode reference
        let mut current_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);

        let mount_name = self
            .mount_point
            .mount_name
            .to_str()
            .map_err(|_| Errnum::ENOTSUP)?;

        let mut search_path = path;

        loop {
            while search_path.starts_with('/') {
                *name_off += 1; // Skip the slash
                search_path = &search_path[1..];
            }

            let len = path_check_new(search_path, &mut is_goal);

            let current_path_segment = &search_path[..len];

            if len == 0 || search_path.is_empty() {
                // Path completely processed
                break;
            }

            search_path = &search_path[len..];

            let r = self.ext4_dir_find_entry(
                &mut current_inode_ref,
                current_path_segment,
                len as u32,
                &mut dir_search_result,
            );
            if let Err(e) = r {
                if e.error() != Errnum::ENOENT.into() || (iflags & O_CREAT) == 0 {
                    return_errno_with_message!(Errnum::ENOENT, "file not found and not create");
                }
                // Handle file or directory creation if allowed
                let new_inode_type = if is_goal {
                    ftype
                } else {
                    DirEntryType::EXT4_DE_DIR.bits()
                };

                let mut new_inode_ref = Ext4InodeRef::new(self.self_ref.clone());
                let r = new_inode_ref.ext4_fs_alloc_inode(new_inode_type);

                if r != EOK {
                    return_errno_with_message!(Errnum::EALLOCFIAL, "alloc inode fail");
                }

                new_inode_ref.ext4_fs_inode_blocks_init();

                let r = self.ext4_link(
                    &mut current_inode_ref,
                    &mut new_inode_ref,
                    current_path_segment,
                    len as u32,
                );

                if r != EOK {
                    /*Fail. Free new inode.*/
                    return_errno_with_message!(Errnum::ELINKFIAL, "link fail");
                }

                self.ext4_fs_put_inode_ref_csum(&mut current_inode_ref);
                self.ext4_fs_put_inode_ref_csum(&mut new_inode_ref);
                self.ext4_fs_put_inode_ref_csum(parent_inode);

                current_inode_ref = new_inode_ref; // Continue with the new inode
                continue;
            }

            *parent_inode = current_inode_ref;

            // Update the current inode to the found directory entry's inode
            current_inode_ref =
                Ext4InodeRef::get_inode_ref(self.self_ref.clone(), dir_search_result.dentry.inode);

            if is_goal {
                break;
            }

            *name_off += len as u32;
        }

        if is_goal {
            file.inode = current_inode_ref.inode_num;
            file.fpos = 0;
            file.fsize = current_inode_ref.inner.inode.inode_get_size();
        }

        Ok(EOK)
    }

    #[allow(unused)]
    pub fn ext4_open_new(
        &self,
        file: &mut Ext4File,
        path: &str,
        flags: &str,
        file_expect: bool,
    ) -> Result<usize> {
        let mut iflags = 0;
        let mut filetype = DirEntryType::EXT4_DE_UNKNOWN;

        // get mount point
        file.mp = self.mount_point.clone();

        // get open flags
        iflags = self.ext4_parse_flags(flags).unwrap();

        // file for dir
        if file_expect {
            filetype = DirEntryType::EXT4_DE_REG_FILE;
        } else {
            filetype = DirEntryType::EXT4_DE_DIR;
        }

        if iflags & O_CREAT != 0 {
            self.ext4_trans_start();
        }

        let mut root_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);

        let mut name_off = 0;
        let r = self.ext4_generic_open2(
            file,
            path,
            iflags,
            filetype.bits(),
            &mut root_inode_ref,
            &mut name_off,
        );
        r
    }

    // read all extent
    #[allow(unused)]
    pub fn ext4_file_read_old(&self, ext4_file: &mut Ext4File) -> Vec<u8> {
        // 创建一个空的向量，用于存储文件的内容
        let mut file_data: Vec<u8> = Vec::new();

        // 创建一个空的向量，用于存储文件的所有extent信息
        let mut extents: Vec<Ext4Extent> = Vec::new();

        let super_block = &self.super_block;

        let inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), ext4_file.inode);

        inode_ref.ext4_find_all_extent(&mut extents);

        // 遍历extents向量，对每个extent，计算它的物理块号，然后调用read_block函数来读取数据块，并将结果追加到file_data向量中
        for extent in extents {
            // 获取extent的起始块号、块数和逻辑块号
            let start_block = extent.start_lo as u64 | ((extent.start_hi as u64) << 32);
            let block_count = extent.block_count as u64;
            let logical_block = extent.first_block as u64;
            // 计算extent的物理块号
            let physical_block = start_block + logical_block;
            // 从file中读取extent的所有数据块，并将结果追加到file_data向量中
            for i in 0..block_count {
                let block_num = physical_block + i;
                let block_data = inode_ref
                    .fs()
                    .block_device
                    .read_offset(block_num as usize * BLOCK_SIZE);
                file_data.extend(block_data);
            }
        }
        file_data
    }

    #[allow(unused)]
    pub fn ext4_file_read(
        &self,
        ext4_file: &mut Ext4File,
        read_buf: &mut [u8],
        size: usize,
        read_cnt: &mut usize,
    ) -> Result<usize> {
        if ext4_file.fpos > ext4_file.fsize as usize {
            log::error!(
                "read offset exceeds file size  fpos {:x?}  fsize {:x?}",
                ext4_file.fpos,
                ext4_file.fsize
            );
            return_errno_with_message!(Errnum::EINVAL, "read offset exceeds file size")
        }

        if size == 0 {
            return Ok(EOK);
        }

        let mut inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), ext4_file.inode);

        // sync file size
        ext4_file.fsize = inode_ref.inner.inode.inode_get_size();

        let is_softlink = inode_ref.inner.inode.ext4_inode_type(&self.super_block)
            == EXT4_INODE_MODE_SOFTLINK as u32;

        if is_softlink {
            log::debug!("ext4_read unsupported softlink");
        }

        let block_size = BLOCK_SIZE;

        // 计算读取大小
        let size_to_read = if size > (ext4_file.fsize as usize - ext4_file.fpos) {
            ext4_file.fsize as usize - ext4_file.fpos
        } else {
            size
        };

        let mut iblock_idx = (ext4_file.fpos / block_size) as u32;
        let iblock_last = ((ext4_file.fpos + size_to_read) / block_size) as u32;
        let mut unalg = (ext4_file.fpos % block_size) as u32;

        let mut offset = 0;
        let mut total_bytes_read = 0;

        if unalg > 0 {
            let first_block_read_len = core::cmp::min(block_size - unalg as usize, size_to_read);
            let mut fblock = 0;

            inode_ref.get_inode_dblk_idx(&mut iblock_idx, &mut fblock, false);

            // if r != EOK {
            //     return Err(Ext4Error::new(r));
            // }

            if fblock != 0 {
                let block_offset = fblock * block_size as u64 + unalg as u64;
                let block_data = self.block_device.read_offset(block_offset as usize);

                // Copy data from block to the user buffer
                read_buf[offset..offset + first_block_read_len]
                    .copy_from_slice(&block_data[0..first_block_read_len]);
            } else {
                // Handle the unwritten block by zeroing out the respective part of the buffer
                for x in &mut read_buf[offset..offset + first_block_read_len] {
                    *x = 0;
                }
            }

            offset += first_block_read_len;
            total_bytes_read += first_block_read_len;
            ext4_file.fpos += first_block_read_len;
            *read_cnt += first_block_read_len;
            iblock_idx += 1;
        }

        // Continue with full block reads
        while total_bytes_read < size_to_read {
            let read_length = core::cmp::min(block_size, size_to_read - total_bytes_read);
            let mut fblock = 0;

            inode_ref.get_inode_dblk_idx(&mut iblock_idx, &mut fblock, false);

            // if r != EOK {
            //     return Err(Ext4Error::new(r));
            // }

            if fblock != 0 {
                let block_data = self
                    .block_device
                    .read_offset((fblock * block_size as u64) as usize);
                read_buf[offset..offset + read_length].copy_from_slice(&block_data[0..read_length]);
            } else {
                // Handle the unwritten block by zeroing out the respective part of the buffer
                for x in &mut read_buf[offset..offset + read_length] {
                    *x = 0;
                }
            }

            offset += read_length;
            total_bytes_read += read_length;
            ext4_file.fpos += read_length;
            *read_cnt += read_length;
            iblock_idx += 1;
        }

        return Ok(EOK);
    }

    pub fn ext4_file_write(&self, ext4_file: &mut Ext4File, data: &[u8], size: usize) {
        let super_block_data = self.block_device.read_offset(BASE_OFFSET);
        let super_block = Ext4Superblock::try_from(super_block_data).unwrap();
        let mut inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), ext4_file.inode);
        let block_size = super_block.block_size() as usize;
        let iblock_last = ext4_file.fpos as usize + size / block_size;
        let mut iblk_idx = ext4_file.fpos as usize / block_size;
        let ifile_blocks = ext4_file.fsize as usize + block_size - 1 / block_size;

        let mut fblk = 0;
        let mut fblock_start = 0;
        let mut fblock_count = 0;

        let mut size = size;
        while size >= block_size {
            while iblk_idx < iblock_last {
                if iblk_idx < ifile_blocks {
                    // ext4_fs_append_inode_dblk(&mut inode_ref, &mut (iblk_idx as u32), &mut fblk);
                    inode_ref.append_inode_dblk(&mut (iblk_idx as u32), &mut fblk);
                }

                iblk_idx += 1;

                if fblock_start == 0 {
                    fblock_start = fblk;
                }
                fblock_count += 1;
            }
            size -= block_size;
        }

        for i in 0..fblock_count {
            let idx = i * BLOCK_SIZE as usize;
            let offset = (fblock_start as usize + i as usize) * BLOCK_SIZE;
            self.block_device
                .write_offset(offset, &data[idx..(idx + BLOCK_SIZE as usize)]);
        }
        // inode_ref.inner.inode.size = fblock_count as u32 * BLOCK_SIZE as u32;
        inode_ref.write_back_inode();
        // let mut inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), ext4_file.inode);
        let mut root_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);
        root_inode_ref.write_back_inode();
    }

    pub fn read_dir_entry(&self, inode: u64) -> Vec<Ext4DirEntry> {
        let mut inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), inode as u32);

        let mut iblock = 0;
        let block_size = inode_ref.fs().super_block.block_size();
        let inode_size = inode_ref.inner.inode.inode_get_size();
        let total_blocks = inode_size as u32 / block_size;
        let mut fblock: Ext4Fsblk = 0;

        let mut entries = Vec::new();

        while iblock < total_blocks {
            inode_ref.get_inode_dblk_idx(&mut iblock, &mut fblock, false);
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

                entries.push(de);
            }
            iblock += 1
        }
        entries
    }

    pub fn ext4_trunc_inode(
        &self,
        inode_ref: &mut Ext4InodeRef,
        new_size: u64,
    ) -> Result<usize> {
        let inode_size = inode_ref.inner.inode.inode_get_size();

        if inode_size > new_size {
            let r = inode_ref.truncate_inode(new_size);
        }

        return_errno_with_message!(Errnum::ENOTSUP, "not support");
    }

    #[allow(unused)]
    pub fn ext4_file_remove(&self, path: &str) -> Result<usize> {
        let mut name_off = 0;

        let mut file = Ext4File::new();

        let mut ptr = Box::new(self.mount_point.clone());

        file.mp = self.mount_point.clone();

        let mut iflags = O_RDONLY;

        let mut filetype = DirEntryType::EXT4_DE_UNKNOWN;

        let mut root_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), 2);

        let r = self.ext4_generic_open2(
            &mut file,
            path,
            iflags,
            filetype.bits(),
            &mut root_inode_ref,
            &mut name_off,
        );

        let child_inode = file.inode;

        self.ext4_file_close(&mut file);

        let mut child_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), child_inode);

        let link_count = child_inode_ref.inner.inode.ext4_inode_get_links_cnt();

        if link_count == 1 {
            let mp = self.mount_point.clone();
            self.ext4_trunc_inode(&mut child_inode_ref, 0x0);
        }

        // after trunc
        let mut child_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), child_inode);

        let mut is_goal = false;
        let p = &path[name_off as usize..];
        let len = path_check_new(p, &mut is_goal);
        let r = self.ext4_unlink(
            &mut root_inode_ref,
            &mut child_inode_ref,
            &p[..len as usize],
            len as u32,
        );

        self.ext4_fs_put_inode_ref_csum(&mut root_inode_ref);

        return Ok(EOK);
    }

    #[allow(unused)]
    pub fn ext4_dir_remove(&self, parent_inode: u32, path: &str) -> Result<usize> {
        /*Check if exist.*/
        let mut name_off = 0;

        let mut file = Ext4File::new();

        file.mp = self.mount_point.clone();

        let mut iflags = O_RDONLY;

        let mut filetype = DirEntryType::EXT4_DE_UNKNOWN;

        let mut parent_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), parent_inode);

        let r = self.ext4_dir_find_entry_new(&mut parent_inode_ref, path)?;

        let mut child_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), r.inode);

        if child_inode_ref.has_children(){
            return_errno_with_message!(Errnum::ENOTSUP, "rm dir with children not supported");
        }

        /* Truncate */
        self.ext4_trunc_inode(&mut child_inode_ref, 0);

        self.ext4_unlink(&mut parent_inode_ref, &mut child_inode_ref, path, path.len() as u32);

        self.ext4_fs_put_inode_ref_csum(&mut parent_inode_ref);


        // to do 

        // ext4_inode_set_del_time
        // ext4_inode_set_links_cnt
        // ext4_fs_free_inode(&child)

        return Ok(EOK);
    }

    #[allow(unused)]
    pub fn ext4_open_from(
        &self,
        parent_inode: u32,
        file: &mut Ext4File,
        path: &str,
        flags: &str,
        file_expect: bool,
    ) -> Result<usize> {
        let mut iflags = 0;
        let mut filetype = DirEntryType::EXT4_DE_UNKNOWN;

        // get mount point
        file.mp = self.mount_point.clone();

        // get open flags
        iflags = self.ext4_parse_flags(flags).unwrap();

        // file for dir
        if file_expect {
            filetype = DirEntryType::EXT4_DE_REG_FILE;
        } else {
            filetype = DirEntryType::EXT4_DE_DIR;
        }

        if iflags & O_CREAT != 0 {
            self.ext4_trans_start();
        }

        // let mut parent_inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), parent_inode);

        let mut name_off = 0;
        let r = self.ext4_generic_open_from(
            file,
            path,
            iflags,
            filetype.bits(),
            parent_inode,
            &mut name_off,
        );
        r
    }

    // with dir search path offset
    pub fn ext4_generic_open_from(
        &self,
        file: &mut Ext4File,
        path: &str,
        iflags: u32,
        ftype: u8,
        parent_inode: u32,
        name_off: &mut u32,
    ) -> Result<usize> {
        let mut is_goal = false;

        let mut data: Vec<u8> = Vec::with_capacity(BLOCK_SIZE);
        let ext4_blk = Ext4Block {
            logical_block_id: 0,
            disk_block_id: 0,
            block_data: &mut data,
            dirty: true,
        };
        let de = Ext4DirEntry::default();
        let mut dir_search_result = Ext4DirSearchResult::new(ext4_blk, de);

        // Load the root inode reference
        let mut current_inode_ref =
            Ext4InodeRef::get_inode_ref(self.self_ref.clone(), parent_inode);

        let mount_name = self
            .mount_point
            .mount_name
            .to_str()
            .map_err(|_| Errnum::ENOTSUP)?;

        let mut search_path = path;

        loop {
            while search_path.starts_with('/') {
                *name_off += 1; // Skip the slash
                search_path = &search_path[1..];
            }

            let len = path_check_new(search_path, &mut is_goal);

            let current_path_segment = &search_path[..len];

            if len == 0 || search_path.is_empty() {
                // Path completely processed
                break;
            }

            search_path = &search_path[len..];

            log::info!("current_path_segment: {:?}", current_path_segment);
            let r = self.ext4_dir_find_entry(
                &mut current_inode_ref,
                current_path_segment,
                len as u32,
                &mut dir_search_result,
            );
            if let Err(e) = r {
                if e.error() != Errnum::ENOENT.into() || (iflags & O_CREAT) == 0 {
                    return_errno_with_message!(Errnum::ENOENT, "file not found and not create");
                }
                // Handle file or directory creation if allowed
                let new_inode_type = if is_goal {
                    ftype
                } else {
                    DirEntryType::EXT4_DE_DIR.bits()
                };

                let mut new_inode_ref = Ext4InodeRef::new(self.self_ref.clone());
                let r = new_inode_ref.ext4_fs_alloc_inode(new_inode_type);

                if r != EOK {
                    return_errno_with_message!(Errnum::EALLOCFIAL, "alloc inode fail");
                }

                new_inode_ref.ext4_fs_inode_blocks_init();

                let r = self.ext4_link(
                    &mut current_inode_ref,
                    &mut new_inode_ref,
                    current_path_segment,
                    len as u32,
                );

                if r != EOK {
                    /*Fail. Free new inode.*/
                    return_errno_with_message!(Errnum::ELINKFIAL, "link fail");
                }

                self.ext4_fs_put_inode_ref_csum(&mut current_inode_ref);
                self.ext4_fs_put_inode_ref_csum(&mut new_inode_ref);
                // self.ext4_fs_put_inode_ref_csum(parent_inode);

                current_inode_ref = new_inode_ref; // Continue with the new inode
                continue;
            }

            // *parent_inode = current_inode_ref;

            // Update the current inode to the found directory entry's inode
            current_inode_ref =
                Ext4InodeRef::get_inode_ref(self.self_ref.clone(), dir_search_result.dentry.inode);

            if is_goal {
                break;
            }

            *name_off += len as u32;
        }

        if is_goal {
            file.inode = current_inode_ref.inode_num;
            file.fpos = 0;
            file.fsize = current_inode_ref.inner.inode.inode_get_size();
        }

        Ok(EOK)
    }
}
