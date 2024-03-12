extern crate alloc;
extern crate log;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;

use super::ext4_defs::*;
use crate::consts::*;
use crate::prelude::*;
use crate::utils::*;

pub(crate) const BASE_OFFSET: usize = 1024;
pub(crate) const BLOCK_SIZE: usize = 4096;

pub trait BlockDevice: Send + Sync + Any + Debug {
    fn read_offset(&self, offset: usize) -> Vec<u8>;
    fn write_offset(&self, offset: usize, data: &[u8]);
}

impl dyn BlockDevice {
    pub fn downcast_ref<T: BlockDevice>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }
}

#[derive(Debug)]
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

impl Ext4 {
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Self> {
        // Load the superblock
        // TODO: if the main superblock is corrupted, should we load the backup?
        let raw_data = block_device.read_offset(BASE_OFFSET);
        let super_block = Ext4Superblock::try_from(raw_data).unwrap();

        // println!("super_block: {:x?}", super_block);
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

    // // 使用libc库定义的常量
    // fn ext4_parse_flags(&self, flags: &str) -> Result<u32> {
    //     let flag = flags.parse::<Ext4OpenFlags>().unwrap(); // 从字符串转换为标志
    //     let file_flags = match flag {
    //         Ext4OpenFlags::ReadOnly => O_RDONLY,
    //         Ext4OpenFlags::WriteOnly => O_WRONLY,
    //         Ext4OpenFlags::WriteCreateTrunc => O_WRONLY | O_CREAT | O_TRUNC,
    //         Ext4OpenFlags::WriteCreateAppend => O_WRONLY | O_CREAT | O_APPEND,
    //         Ext4OpenFlags::ReadWrite => O_RDWR,
    //         Ext4OpenFlags::ReadWriteCreateTrunc => O_RDWR | O_CREAT | O_TRUNC,
    //         Ext4OpenFlags::ReadWriteCreateAppend => O_RDWR | O_CREAT | O_APPEND,
    //     };
    //     Ok(file_flags as u32) // 转换为数值
    // }

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

    pub fn ext4_open(&self, file: &mut Ext4File, path: &str, flags: &str, file_expect: bool) {
        let mut iflags = 0;
        let mut filetype = DirEntryType::EXT4_DE_UNKNOWN;

        // get mount point
        let mut ptr = Box::new(self.mount_point.clone());
        file.mp = Box::as_mut(&mut ptr) as *mut Ext4MountPoint;

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

        // println!("filetype {:x?}", filetype.bits());
        self.ext4_generic_open(file, path, iflags, filetype.bits(), &mut root_inode_ref);
    }

    pub fn ext4_generic_open(
        &self,
        file: &mut Ext4File,
        path: &str,
        iflags: u32,
        ftype: u8,
        parent_inode: &mut Ext4InodeRef,
    ) {
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

            // println!(
            //     "search_path {:?} len {:?} is_goal {:?}",
            //     search_path, len, is_goal
            // );

            let r = ext4_dir_find_entry(
                &mut search_parent,
                &search_path[..len as usize],
                len as u32,
                &mut dir_search_result,
            );

            // println!("dir_search_result.dentry {:?} r {:?}", dir_search_result.dentry, r);
            if r != EOK {
                // ext4_dir_destroy_result(&mut root_inode_ref, &mut dir_search_result);

                if r != ENOENT {
                    break;
                }

                if !((iflags & O_CREAT) != 0) {
                    println!("error flags not O_CREAT");
                    break;
                }

                let mut child_inode_ref = Ext4InodeRef::new(self.self_ref.clone());

                let mut r;

                if is_goal {
                    r = ext4_fs_alloc_inode(&mut child_inode_ref, ftype);
                } else {
                    r = ext4_fs_alloc_inode(&mut child_inode_ref, DirEntryType::EXT4_DE_DIR.bits())
                }

                if r != EOK {
                    break;
                }

                ext4_fs_inode_blocks_init(&mut child_inode_ref);

                let r = ext4_link(
                    &mut search_parent,
                    &mut child_inode_ref,
                    &search_path[..len as usize],
                    len as u32,
                );

                // println!("is_goal {:?} r {:?}", is_goal, r);

                // let mut data: Vec<u8> = Vec::with_capacity(BLOCK_SIZE);
                // let ext4_blk = Ext4Block {
                //     logical_block_id: 0,
                //     disk_block_id: 0,
                //     block_data: &mut data,
                //     dirty: true,
                // };
                // let mut de = Ext4DirEntry::default();
                // let mut test = Ext4DirSearchResult::new(ext4_blk, de);

                // let r = ext4_dir_find_entry(
                //     &mut search_parent,
                //     &search_path[..len as usize],
                //     len as u32,
                //     &mut test,
                // );
                // let name = get_name(
                //     test.dentry.name,
                //     test.dentry.name_len as usize,
                // )
                // .unwrap();
                // println!("create after link search de name{:?} de inode {:x?} result {:?}", name, test.dentry.inode, r);


                if r != EOK {
                    /*Fail. Free new inode.*/
                    break;
                }

                ext4_fs_put_inode_ref_csum(&mut search_parent);
                ext4_fs_put_inode_ref_csum(&mut child_inode_ref);
                ext4_fs_put_inode_ref_csum(parent_inode);                

                continue;
            }

            let name = get_name(
                dir_search_result.dentry.name,
                dir_search_result.dentry.name_len as usize,
            )
            .unwrap();
            // println!("find de name{:?} de inode {:x?}", name, dir_search_result.dentry.inode);

            if is_goal {
                file.inode = dir_search_result.dentry.inode;
                return;
            } else {
                search_parent = Ext4InodeRef::get_inode_ref(
                    self.self_ref.clone(),
                    dir_search_result.dentry.inode,
                );
                search_path = &search_path[len..];
            }
        }
    }

    pub fn ext4_file_read(&self, ext4_file: &mut Ext4File) -> Vec<u8> {
        // 创建一个空的向量，用于存储文件的内容
        let mut file_data: Vec<u8> = Vec::new();

        // 创建一个空的向量，用于存储文件的所有extent信息
        let mut extents: Vec<Ext4Extent> = Vec::new();

        let super_block = &self.super_block;

        let inode_ref = Ext4InodeRef::get_inode_ref(self.self_ref.clone(), ext4_file.inode);

        ext4_find_all_extent(&inode_ref, &mut extents);

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
                    ext4_fs_append_inode_dblk(&mut inode_ref, &mut (iblk_idx as u32), &mut fblk);
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
}

pub fn ext4_fs_put_inode_ref_csum(inode_ref: &mut Ext4InodeRef) {
    inode_ref.write_back_inode();
}

pub fn ext4_fs_put_inode_ref(inode_ref: &mut Ext4InodeRef) {
    inode_ref.write_back_inode_without_csum();
}

pub fn ext4_link(
    parent: &mut Ext4InodeRef,
    child: &mut Ext4InodeRef,
    name: &str,
    name_len: u32,
) -> usize {
    // println!("link parent inode {:x?} child inode {:x?} name {:?}", parent.inode_num, child.inode_num, name);
    /* Add entry to parent directory */
    let r = ext4_dir_add_entry(parent, child, name, name_len);

    /* Fill new dir -> add '.' and '..' entries.
     * Also newly allocated inode should have 0 link count.
    	*/
    let mut is_dir = false;
    if child.inner.inode.mode & EXT4_INODE_MODE_TYPE_MASK as u16 == EXT4_INODE_MODE_DIRECTORY as u16
    {
        is_dir = true;
    }

    if is_dir {
        // add '.' and '..' entries
        let fs = child.fs().self_ref.clone();
        let mut child_inode_ref = Ext4InodeRef::new(fs);
        child_inode_ref.inode_num = child.inode_num;
        child_inode_ref.inner.inode = child.inner.inode.clone();

        let r = ext4_dir_add_entry(&mut child_inode_ref, child, ".", 1);
        child.inner.inode.size = child_inode_ref.inner.inode.size;
        child.inner.inode.block = child_inode_ref.inner.inode.block;
        let r = ext4_dir_add_entry(&mut child_inode_ref, parent, "..", 2);

        child.inner.inode.links_count = 2;
        parent.inner.inode.links_count += 1;

        return EOK;
    }

    child.inner.inode.links_count += 1;
    EOK
}

pub fn ext4_dir_add_entry(
    parent: &mut Ext4InodeRef,
    child: &mut Ext4InodeRef,
    path: &str,
    len: u32,
) -> usize {
    let mut iblock = 0;
    let block_size = parent.fs().super_block.block_size();
    let inode_size = parent.inner.inode.ext4_inode_get_size();
    // let inode_size = parent.fs().super_block.inode_size_file(&parent.inner.inode);
    let total_blocks = inode_size as u32 / block_size;
    let mut success = false;

    let mut fblock: ext4_fsblk_t = 0;

    // println!("ext4_dir_add_entry parent inode {:x?} inode_size {:x?}", parent.inode_num, inode_size);
    while iblock < total_blocks {
        ext4_fs_get_inode_dblk_idx(parent, &mut iblock, &mut fblock, false);

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

        let r = ext4_dir_try_insert_entry(parent, &mut ext4_block, child, path, len);

        if r == EOK {
            success = true;
            return EOK;
            // break;
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

        let r = ext4_dir_find_in_block(&mut ext4_block, path, len, &mut dir_search_result);

        if r {
            return EOK;
        }

        iblock += 1;
    }

    /* No free block found - needed to allocate next data block */
    iblock = 0;
    fblock = 0;

    ext4_fs_append_inode_dblk(parent, &mut (iblock as u32), &mut fblock);

    /* Load new block */
    let block_device = parent.fs().block_device.clone();
    let mut data = block_device.read_offset(fblock as usize * BLOCK_SIZE);
    let mut ext4_block = Ext4Block {
        logical_block_id: iblock,
        disk_block_id: fblock,
        block_data: &mut data,
        dirty: false,
    };

    let mut new_entry = Ext4DirEntry::default();
    let el = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
    ext4_dir_write_entry(&mut new_entry, el as u16, &child, path, len);

    copy_dir_entry_to_array(&new_entry, &mut ext4_block.block_data, 0);

    // init tail
    let ptr = ext4_block.block_data.as_mut_ptr();
    let mut tail = unsafe{*(ptr.add(BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>()) as *mut Ext4DirEntryTail)};
    tail.rec_len = size_of::<Ext4DirEntryTail>() as u16;
    tail.reserved_ft = 0xDE;
    tail.reserved_zero1 = 0;
    tail.reserved_zero2 = 0;


    tail.ext4_dir_set_csum(
        &parent.fs().super_block,
        &new_entry,
        &ext4_block.block_data[..],
    );


    let tail_offset = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
    copy_diren_tail_to_array(&tail, &mut ext4_block.block_data, tail_offset);

    tail.ext4_dir_set_csum(
        &parent.fs().super_block,
        &new_entry,
        &ext4_block.block_data[..],
    );

    ext4_block.sync_blk_to_disk(block_device.clone());

	// struct ext4_block b;

    EOK
}

pub fn ext4_dir_try_insert_entry(
    parent: &Ext4InodeRef,
    dst_blk: &mut Ext4Block,
    child: &mut Ext4InodeRef,
    name: &str,
    name_len: u32,
) -> usize {
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
                ext4_dir_write_entry(&mut new_entry, free_space as u16, &child, name, name_len);

                // update parent new_de to blk_data
                copy_dir_entry_to_array(&de, &mut dst_blk.block_data, offset);
                copy_dir_entry_to_array(&new_entry, &mut dst_blk.block_data, offset + sz);

                // set tail csum
                let mut tail = Ext4DirEntryTail::from(&mut dst_blk.block_data, BLOCK_SIZE).unwrap();
                let block_device = parent.fs().block_device.clone();
                tail.ext4_dir_set_csum(
                    &parent.fs().super_block,
                    &de,
                    &dst_blk.block_data[offset..],
                );

                let parent_de = Ext4DirEntry::try_from(&dst_blk.block_data[..]).unwrap();
                tail.ext4_dir_set_csum(
                    &parent.fs().super_block,
                    &parent_de,
                    &dst_blk.block_data[..],
                );

                let tail_offset = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
                copy_diren_tail_to_array(&tail, &mut dst_blk.block_data, tail_offset);

                // sync to disk
                dst_blk.sync_blk_to_disk(block_device.clone());

                return EOK;
            }
        }
        offset = offset + de.entry_len as usize;
    }

    ENOSPC
}

// 写入一个ext4目录项
pub fn ext4_dir_write_entry(
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
        _ => println!("{}: unknown type", file_type),
    }

    en.inode = child.inode_num;
    en.entry_len = entry_len;
    en.name_len = name_len as u8;

    let en_name_ptr = en.name.as_mut_ptr();
    unsafe {
        en_name_ptr.copy_from_nonoverlapping(name.as_ptr(), name_len as usize);
    }
    let name = get_name(en.name, en.name_len as usize).unwrap();
    // println!("ext4_dir_write_entry name {:?}", name);
}

pub fn ext4_fs_append_inode_dblk(
    inode_ref: &mut Ext4InodeRef,
    iblock: &mut ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
) {
    let inode_size = inode_ref.inner.inode.ext4_inode_get_size();
    let block_size = BLOCK_SIZE as u64;

    *iblock = ((inode_size + block_size - 1) / block_size) as u32;

    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;
    ext4_extent_get_blocks(inode_ref, *iblock, 1, &mut current_fsblk, true, &mut 0);

    current_block = current_fsblk;
    *fblock = current_block;

    inode_ref
        .inner
        .inode
        .ext4_inode_set_size(inode_size + BLOCK_SIZE as u64);

    inode_ref.write_back_inode();

    // let mut inode_ref = Ext4InodeRef::get_inode_ref(inode_ref.fs().self_ref.clone(), inode_ref.inode_num);

    // println!("ext4_fs_append_inode_dblk inode {:x?} inode_size {:x?}", inode_ref.inode_num, inode_ref.inner.inode.size);
    // println!("fblock {:x?}", fblock);
}

pub fn ext4_fs_inode_blocks_init(inode_ref: &mut Ext4InodeRef) {
    // println!(
    //     "ext4_fs_inode_blocks_init mode {:x?}",
    //     inode_ref.inner.inode.mode
    // );

    let mut inode = &mut inode_ref.inner.inode;

    let mode = inode.mode;

    let inode_type = InodeMode::from_bits(mode & EXT4_INODE_MODE_TYPE_MASK as u16).unwrap();

    match inode_type {
        InodeMode::S_IFDIR => {}
        InodeMode::S_IFREG => {}
        /* Reset blocks array. For inode which is not directory or file, just
         * fill in blocks with 0 */
        _ => {
            println!("inode_type {:?}", inode_type);
            return;
        }
    }

    /* Initialize extents */
    inode.ext4_inode_set_flags(EXT4_INODE_FLAG_EXTENTS as u32);

    /* Initialize extent root header */
    inode.ext4_extent_tree_init();
    // println!("inode iblock {:x?}", inode.block);

    // inode_ref.dirty = true;
}

pub fn ext4_fs_alloc_inode(child_inode_ref: &mut Ext4InodeRef, filetype: u8) -> usize {
    let mut is_dir = false;

    let inode_size = child_inode_ref.fs().super_block.inode_size();
    let extra_size = child_inode_ref.fs().super_block.extra_size();

    if filetype == DirEntryType::EXT4_DE_DIR.bits() {
        is_dir = true;
    }

    let mut index = 0;
    let rc = ext4_ialloc_alloc_inode(child_inode_ref.fs(), &mut index, is_dir);

    child_inode_ref.inode_num = index;

    let mut inode = &mut child_inode_ref.inner.inode;

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
        // println!("ext4_fs_correspond_inode_mode {:x?}", ext4_fs_correspond_inode_mode(filetype));
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
pub fn ext4_dir_destroy_result(inode_ref: &mut Ext4InodeRef, result: &mut Ext4DirSearchResult) {
    result.block.logical_block_id = 0;
    result.block.disk_block_id = 0;
    result.dentry = Ext4DirEntry::default();
}

pub fn ext4_dir_find_entry(
    parent: &mut Ext4InodeRef,
    name: &str,
    name_len: u32,
    result: &mut Ext4DirSearchResult,
) -> usize {
    // println!("ext4_dir_find_entry parent {:x?} {:?}",parent.inode_num,  name);
    let mut iblock = 0;
    let mut fblock: ext4_fsblk_t = 0;

    let inode_size: u32 = parent.inner.inode.size;
    let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

    while iblock < total_blocks {
        ext4_fs_get_inode_dblk_idx(parent, &mut iblock, &mut fblock, false);

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

        let r = ext4_dir_find_in_block(&mut ext4_block, name, name_len, result);
        if r {
            return EOK;
        }

        iblock += 1
    }

    ENOENT
}

pub fn ext4_extent_get_blocks(
    inode_ref: &mut Ext4InodeRef,
    iblock: ext4_lblk_t,
    max_blocks: u32,
    result: &mut ext4_fsblk_t,
    create: bool,
    blocks_count: &mut u32,
) {
    *result = 0;
    *blocks_count = 0;
    
    let mut path: Option<Vec<Ext4ExtentPath>> = None;
    let err = ext4_find_extent(inode_ref, iblock, &mut path, 0);

    let inode = &inode_ref.inner.inode;
    // 确认ext4_find_extent成功执行
    if err != EOK {
        return;
    }

    let depth = unsafe { *ext4_inode_hdr(inode) }.depth as usize;
    let mut path = path.unwrap();

    if !path[depth].extent.is_null() {
        let ex = unsafe { *path[depth].extent };
        let ee_block = ex.first_block;
        let ee_start = ext4_ext_pblock(&ex);
        let ee_len = ext4_ext_get_actual_len(&ex);

        if iblock >= ee_block && iblock < ee_block + ee_len as u32 {
            let allocated = ee_len - (iblock - ee_block) as u16;
            *blocks_count = allocated as u32;

            if !create || ext4_ext_is_unwritten(&ex) {
                *result = (iblock - ee_block + ee_start) as u64;
                return;
            }
        }
    }

    // 如果没有找到对应的extent，并且create为true，则需要分配和插入新的extent
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
        ext4_balloc_alloc_block(inode_ref, goal as u64, &mut alloc_block);

        *result = alloc_block;

        // 创建并插入新的extent
        newex.first_block = iblock;
        newex.start_lo = alloc_block as u32 & 0xffffffff;
        newex.start_hi = (((alloc_block as u32) << 31) << 1) as u16;
        newex.block_count = allocated as u16;

        ext4_ext_insert_extent(inode_ref, &mut path[0], &newex, 0);
    }
}

pub fn ext_inode_hdr(inode: &Ext4Inode) -> *const Ext4ExtentHeader {
    let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;
    eh
}

pub fn ext_depth(inode: &Ext4Inode) -> u16 {
    let header = ext_inode_hdr(inode);
    unsafe { (*header).depth }
}

pub fn ext_last_extent(eh: *const Ext4ExtentHeader) -> *mut Ext4Extent {
    // 如果头部为空，返回空指针
    if eh.is_null() {
        return ptr::null_mut();
    }

    // 获取头部的extent数
    let count = unsafe { (*eh).entries_count };

    // 如果extent数为0，返回空指针
    if count == 0 {
        return ptr::null_mut();
    }

    // 获取头部中第一个extent的指针
    let first = ext_first_extent(eh);

    // 返回头部中最后一个extent的指针，即第一个extent的指针加上extent数减一
    return unsafe { first.add((count - 1) as usize) };
}

// ext_first_extent函数
pub fn ext_first_extent(eh: *const Ext4ExtentHeader) -> *mut Ext4Extent {
    // 如果头部为空，返回空指针
    if eh.is_null() {
        return ptr::null_mut();
    }

    // // 获取头部的extent数
    // let count = unsafe { (*eh).entries_count };

    // // 如果extent数为0，返回空指针
    // if count == 0 {
    //     return ptr::null_mut();
    // }

    // 返回头部中第一个extent的指针，即头部的指针加上头部的大小
    return unsafe { (eh as *mut u8).add(mem::size_of::<Ext4ExtentHeader>()) as *mut Ext4Extent };
}

pub fn ext4_ext_pblock(ex: &Ext4Extent) -> u32 {
    let mut block = 0;

    unsafe {
        block = ex.start_lo;
        block |= ((ex.start_hi as u32) << 31) << 1;
    }

    block
}

pub fn ext4_ext_is_unwritten(ext: &Ext4Extent) -> bool {
    // 返回extent是否是未写入的
    ext.block_count > EXT_INIT_MAX_LEN
}

pub fn ext4_ext_get_actual_len(ext: &Ext4Extent) -> u16 {
    // 返回extent的实际长度
    if ext.block_count <= EXT_INIT_MAX_LEN {
        ext.block_count
    } else {
        ext.block_count - EXT_INIT_MAX_LEN
    }
}

pub fn ext4_ext_mark_unwritten(ext: *mut Ext4Extent) {
    unsafe {
        (*ext).block_count |= EXT_INIT_MAX_LEN;
    }
}

pub fn ext4_ext_can_append(ex1: &Ext4Extent, ex2: &Ext4Extent) -> bool {
    // 检查是否可以将ex2合并到ex1的后面
    // println!(
    //     "\n\npblock1={:x?} pblock2={:x?}",
    //     ext4_ext_pblock(ex1),
    //     ext4_ext_pblock(ex2)
    // );
    // println!(
    //     "len1={:x?} len2={:x?}",
    //     ext4_ext_get_actual_len(ex1),
    //     ext4_ext_get_actual_len(ex2)
    // );
    // println!(
    //     "first_block1={:x?} first_block2={:x?}",
    //     ex1.first_block, ex2.first_block
    // );
    if ext4_ext_pblock(ex1) + ext4_ext_get_actual_len(ex1) as u32 != ext4_ext_pblock(ex2) {
        return false;
    }

    if ext4_ext_is_unwritten(ex1) {
        if ext4_ext_get_actual_len(ex1) + ext4_ext_get_actual_len(ex2) > EXT_UNWRITTEN_MAX_LEN {
            return false;
        }
    } else if ext4_ext_get_actual_len(ex1) + ext4_ext_get_actual_len(ex2) > EXT_INIT_MAX_LEN {
        return false;
    }

    // 检查逻辑块号是否连续
    if ex1.first_block + ext4_ext_get_actual_len(ex1) as u32 != ex2.first_block {
        return false;
    }
    return true;
}

pub fn ext4_ext_can_prepend(ex1: &Ext4Extent, ex2: &Ext4Extent) -> bool {
    // 检查是否可以将ex2合并到ex1的前面
    if ext4_ext_pblock(ex2) + ext4_ext_get_actual_len(ex2) as u32 != ext4_ext_pblock(ex1) {
        return false;
    }
    if ext4_ext_is_unwritten(ex1) {
        if ext4_ext_get_actual_len(ex1) + ext4_ext_get_actual_len(ex2) > EXT_UNWRITTEN_MAX_LEN {
            return false;
        }
    } else if ext4_ext_get_actual_len(ex1) + ext4_ext_get_actual_len(ex2) > EXT_INIT_MAX_LEN {
        return false;
    }

    // 检查逻辑块号是否连续
    if ex2.first_block + ext4_ext_get_actual_len(ex2) as u32 != ex1.first_block {
        return false;
    }

    // 如果以上条件都满足，返回true
    true
}

pub fn ext4_ext_insert_extent(
    inode_ref: &mut Ext4InodeRef,
    path: &mut Ext4ExtentPath,
    newext: &Ext4Extent,
    flags: i32,
) {
    let mut depth = ext_depth(&inode_ref.inner.inode);
    let mut need_split = false;

    ext4_ext_insert_leaf(inode_ref, path, depth, newext, flags, &mut need_split);

    inode_ref.write_back_inode_without_csum();
}

pub fn ext4_ext_insert_leaf(
    inode_ref: &mut Ext4InodeRef,
    path: &mut Ext4ExtentPath,
    depth: u16,
    newext: &Ext4Extent,
    flags: i32,
    need_split: &mut bool,
) -> usize {
    let eh = path.header;
    let ex = path.extent;
    let last_ex = ext_last_extent(eh);

    let mut diskblock = 0;
    diskblock = newext.start_lo;
    diskblock |= ((newext.start_hi as u32) << 31) << 1;

    unsafe {
        if !ex.is_null() && ext4_ext_can_append(&*(path.extent), newext) {
            if ext4_ext_is_unwritten(&*(path.extent)) {
                ext4_ext_mark_unwritten((*path).extent);
            }
            (*(path.extent)).block_count =
                ext4_ext_get_actual_len(&*(path.extent)) + ext4_ext_get_actual_len(&newext);
            (*path).p_block = diskblock as u64;
            return EOK;
        }

        if !ex.is_null() && ext4_ext_can_prepend(&*(path.extent), newext) {
            (*(path.extent)).block_count =
                ext4_ext_get_actual_len(&*(path.extent)) + ext4_ext_get_actual_len(&newext);
            (*path).p_block = diskblock as u64;

            if ext4_ext_is_unwritten(&*(path.extent)) {
                ext4_ext_mark_unwritten((*path).extent);
            }
            return EOK;
        }
    }

    if ex.is_null() {
        let first_extent = ext_first_extent(eh);
        unsafe {
            (*path).extent = first_extent;
            // println!("first_extent {:x?}", unsafe{*first_extent});
        }
        unsafe {
            if (*eh).entries_count == (*eh).max_entries_count {
                *need_split = true;
                return EIO;
            } else {
                *(path.extent) = *newext;
            }
        }
    }

    unsafe {
        if (*eh).entries_count == (*eh).max_entries_count {
            *need_split = true;
            *(path.extent) = *newext;

            (*path).p_block = diskblock as u64;
            return EIO;
        } else {
            if ex.is_null() {
                let first_extent = ext_first_extent(eh);
                (*path).extent = first_extent;
                *(path.extent) = *newext;
            } else if newext.first_block > (*(path.extent)).first_block {
                // insert after
                let next_extent = ex.add(1);
                (*path).extent = next_extent;
            } else {
            }
        }

        *(path.extent) = *newext;
        (*eh).entries_count += 1;
    }
    unsafe {
        *(path.extent) = *newext;
    }

    return EOK;
}

pub fn ext4_find_all_extent(inode_ref: &Ext4InodeRef, extents: &mut Vec<Ext4Extent>) {
    let extent_header = Ext4ExtentHeader::try_from(&inode_ref.inner.inode.block[..2]).unwrap();
    // println!("extent_header {:x?}", extent_header);
    let data = &inode_ref.inner.inode.block;
    let depth = extent_header.depth;

    ext4_add_extent(inode_ref, depth, data, extents, true);
}

pub fn ext4_add_extent(
    inode_ref: &Ext4InodeRef,
    depth: u16,
    data: &[u32],
    extents: &mut Vec<Ext4Extent>,
    first_level: bool,
) {
    let extent_header = Ext4ExtentHeader::try_from(data).unwrap();
    let extent_entries = extent_header.entries_count;
    // println!("extent_entries {:x?}", extent_entries);
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
        let data = inode_ref
            .fs()
            .block_device
            .read_offset(block as usize * BLOCK_SIZE);
        let data: Vec<u32> = unsafe { core::mem::transmute(data) };
        ext4_add_extent(inode_ref, depth - 1, &data, extents, false);
    }
}

pub fn ext4_idx_pblock(idx: *mut Ext4ExtentIndex) -> u64 {
    // 如果索引为空，返回0
    if idx.is_null() {
        return 0;
    }
    // 获取索引的低32位物理块号
    let mut pblock = unsafe { (*idx).leaf_lo } as u64;

    // 如果支持64位物理块号，获取索引的高16位物理块号
    let pblock_hi = unsafe { (*idx).leaf_hi };
    pblock |= ((pblock_hi as ext4_fsblk_t) << 32) as u64;
    // }

    // 返回索引的物理块号
    return pblock;
}

fn ext4_find_extent(
    inode_ref: &mut Ext4InodeRef,
    block: ext4_lblk_t,
    orig_path: &mut Option<Vec<Ext4ExtentPath>>,
    flags: u32,
) -> usize {
    let inode = &inode_ref.inner.inode;
    let mut eh: &Ext4ExtentHeader;
    let mut buf_block: ext4_fsblk_t = 0;
    let mut path = orig_path.take(); // Take the path out of the Option, which may replace it with None
    let depth = unsafe { *ext4_inode_hdr(inode) }.depth;

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
        ext4_ext_binsearch_idx(&mut path[ppos], block);
        path[ppos].p_block = ext4_idx_pblock(path[ppos].index);
        path[ppos].depth = i;
        path[ppos].extent = core::ptr::null_mut();
        buf_block = path[ppos].p_block;

        i -= 1;
        ppos += 1;
    }

    path[ppos].depth = i;
    path[ppos].extent = core::ptr::null_mut();
    path[ppos].index = core::ptr::null_mut();

    ext4_ext_binsearch(&mut path[ppos], block);
    if !path[ppos].extent.is_null() {
        path[ppos].p_block = ext4_ext_pblock(&unsafe { *(path[ppos].extent) }) as u64;
    }

    *orig_path = Some(path.clone());

    EOK
}

pub fn ext4_ext_find_extent(eh: *mut Ext4ExtentHeader, block: ext4_lblk_t) -> *mut Ext4Extent {
    // 初始化一些变量
    let mut low: i32;
    let mut high: i32;
    let mut mid: i32;
    let mut ex: *mut Ext4Extent;

    // 如果头部的extent数为0，返回空指针
    if eh.is_null() || unsafe { (*eh).entries_count } == 0 {
        return ptr::null_mut();
    }

    // 从头部获取第一个extent的指针
    ex = ext4_first_extent_mut(eh);

    // 如果头部的深度不为0，返回空指针
    if unsafe { (*eh).depth } != 0 {
        return ptr::null_mut();
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
        return ptr::null_mut();
    } else {
        return unsafe { ex.add(high as usize) };
    }
}
pub fn ext4_fs_get_inode_dblk_idx(
    inode_ref: &mut Ext4InodeRef,
    iblock: &mut ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
    extent_create: bool,
) -> usize {
    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;

    let mut blocks_count = 0;
    ext4_extent_get_blocks(
        inode_ref,
        *iblock,
        1,
        &mut current_fsblk,
        false,
        &mut blocks_count,
    );

    current_block = current_fsblk;
    *fblock = current_block;

    EOK
}

pub fn ext4_fs_get_inode_dblk_idx_internal(
    inode_ref: &mut Ext4InodeRef,
    iblock: &mut ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
    extent_create: bool,
    support_unwritten: bool,
) {
    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;

    let mut blocks_count = 0;
    ext4_extent_get_blocks(
        inode_ref,
        *iblock,
        1,
        &mut current_fsblk,
        extent_create,
        &mut blocks_count,
    );
}

pub fn ext4_dir_find_in_block(
    block: &Ext4Block,
    name: &str,
    name_len: u32,
    result: &mut Ext4DirSearchResult,
) -> bool {
    let mut offset = 0;

    while offset < block.block_data.len() {



        let de = Ext4DirEntry::try_from(&block.block_data[offset..]).unwrap();

        offset = offset + de.entry_len as usize;
        if de.inode == 0 {
            continue;
        }

        let s = get_name(de.name, de.name_len as usize);
        // println!("ext4_dir_find_in_block {:?}", s);
        if let Ok(s) = s {
            if name_len == de.name_len as u32 {
                if name.to_string() == s {
                    result.dentry = de;
                    return true;
                }
            }
        }
    }

    false
}

pub fn ext4_ialloc_alloc_inode(fs: Arc<Ext4>, index: &mut u32, is_dir: bool) {
    let mut bgid = fs.last_inode_bg_id;
    let bg_count = fs.super_block.block_groups_count();

    while bgid <= bg_count {
        if bgid == bg_count {
            bgid = 0;
            continue;
        }

        let block_device = fs.block_device.clone();

        let mut raw_data = fs.block_device.read_offset(BASE_OFFSET);
        let mut super_block = Ext4Superblock::try_from(raw_data).unwrap();

        let mut bg =
            Ext4BlockGroup::load(block_device.clone(), &super_block, bgid as usize).unwrap();

        let mut free_inodes = bg.get_free_inodes_count();
        let mut used_dirs = bg.get_used_dirs_count(&super_block);

        if free_inodes > 0 {
            let inode_bitmap_block = bg.get_inode_bitmap_block(&super_block);

            let mut raw_data = fs
                .block_device
                .read_offset(inode_bitmap_block as usize * BLOCK_SIZE);

            let inodes_in_bg = super_block.get_inodes_in_group_cnt(bgid);

            let bitmap_size: u32 = inodes_in_bg / 0x8;

            let mut bitmap_data = &mut raw_data[..bitmap_size as usize];

            let mut idx_in_bg = 0 as u32;

            ext4_bmap_bit_find_clr(bitmap_data, 0, inodes_in_bg, &mut idx_in_bg);
            ext4_bmap_bit_set(&mut bitmap_data, idx_in_bg);

            // update bitmap in disk
            fs.block_device
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

            // println!("alloc inode {:x?}", inode_num);
            return;
        }

        bgid += 1;
    }
    println!("no free inode");
}

pub fn ext4_balloc_alloc_block(
    inode_ref: &mut Ext4InodeRef,
    goal: ext4_fsblk_t,
    fblock: &mut ext4_fsblk_t,
) {
    // let mut alloc: ext4_fsblk_t = 0;
    // let mut bmp_blk_adr: ext4_fsblk_t;
    // let mut rel_blk_idx: u32 = 0;
    // let mut free_blocks: u64;
    // let mut r: i32;

    let fs = inode_ref.fs();

    let block_device = fs.block_device.clone();

    let super_block_data = block_device.read_offset(BASE_OFFSET);
    let mut super_block = Ext4Superblock::try_from(super_block_data).unwrap();

    // let inodes_per_group = super_block.inodes_per_group();
    let blocks_per_group = super_block.blocks_per_group();

    let bgid = goal / blocks_per_group as u64;
    let idx_in_bg = goal % blocks_per_group as u64;

    let mut bg = Ext4BlockGroup::load(block_device.clone(), &super_block, bgid as usize).unwrap();

    let block_bitmap_block = bg.get_block_bitmap_block(&super_block);
    let mut raw_data = block_device.read_offset(block_bitmap_block as usize * BLOCK_SIZE);
    let mut data: &mut Vec<u8> = &mut raw_data;
    let mut rel_blk_idx = 0 as u32;

    ext4_bmap_bit_find_clr(data, idx_in_bg as u32, 0x8000, &mut rel_blk_idx);
    *fblock = rel_blk_idx as u64;
    ext4_bmap_bit_set(&mut data, rel_blk_idx);

    bg.set_block_group_balloc_bitmap_csum(&super_block, &data);
    block_device.write_offset(block_bitmap_block as usize * BLOCK_SIZE, &data);

    /* Update superblock free blocks count */
    let mut super_blk_free_blocks = super_block.free_blocks_count();
    // super_blk_free_blocks -= 1;
    super_block.set_free_blocks_count(super_blk_free_blocks);
    super_block.sync_to_disk(block_device.clone());

    /* Update inode blocks (different block size!) count */
    let mut inode_blocks = inode_ref.inner.inode.ext4_inode_get_blocks_count();
    inode_blocks += 8;
    inode_ref
        .inner
        .inode
        .ext4_inode_set_blocks_count(inode_blocks as u32);
    inode_ref.write_back_inode();

    /* Update block group free blocks count */
    let mut fb_cnt = bg.get_free_blocks_count();
    fb_cnt -= 1;
    bg.set_free_blocks_count(fb_cnt);
    bg.sync_to_disk_with_csum(block_device, bgid as usize, &super_block);
}
