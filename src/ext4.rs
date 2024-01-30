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

// 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
pub fn ext4_ext_binsearch(path: &mut Ext4ExtentPath, block: u32) -> bool {
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

        let ext4 = Arc::new_cyclic(|weak_ref| Self {
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
        let flag = flags.parse::<Ext4OpenFlags>().unwrap(); // 从字符串转换为标志
        let file_flags = match flag {
            Ext4OpenFlags::ReadOnly => O_RDONLY,
            Ext4OpenFlags::WriteOnly => O_WRONLY,
            Ext4OpenFlags::WriteCreateTrunc => O_WRONLY | O_CREAT | O_TRUNC,
            Ext4OpenFlags::WriteCreateAppend => O_WRONLY | O_CREAT | O_APPEND,
            Ext4OpenFlags::ReadWrite => O_RDWR,
            Ext4OpenFlags::ReadWriteCreateTrunc => O_RDWR | O_CREAT | O_TRUNC,
            Ext4OpenFlags::ReadWriteCreateAppend => O_RDWR | O_CREAT | O_APPEND,
        };
        Ok(file_flags as u32) // 转换为数值
    }

    // start transaction
    pub fn ext4_trans_start(&self) {}

    // stop transaction
    pub fn ext4_trans_abort(&self) {}

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
        self.ext4_generic_open(file, path, iflags, filetype.bits(), None);
    }

    pub fn ext4_generic_open(
        &self,
        file: &mut Ext4File,
        path: &str,
        iflags: u32,
        ftype: u8,
        parent_inode: Option<&mut Ext4InodeRef>,
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

        if !parent_inode.is_none() {
            parent_inode.unwrap().inode_num = root_inode_ref.inode_num;
        }

        // search dir
        let mut search_path = ext4_path_skip(&path, ".");
        let mut len = 0;
        loop {
            search_path = ext4_path_skip(search_path, "/");
            len = ext4_path_check(search_path, &mut is_goal);

            // println!("search_path {:?} len {:?} is_goal {:?}", search_path, len, is_goal);

            let r = ext4_dir_find_entry(
                &mut root_inode_ref,
                &search_path[..len as usize],
                len as u32,
                &mut dir_search_result,
            );

            if r != EOK {
                ext4_dir_destroy_result();

                if r != ENOENT {
                    break;
                }

                if (iflags & O_CREAT) != 1 {
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

                let r = ext4_link();

                if r != EOK {
                    /*Fail. Free new inode.*/
                    break;
                }
                ext4_fs_put_inode_ref(&mut child_inode_ref);
            }

            let name = get_name(
                dir_search_result.dentry.name,
                dir_search_result.dentry.name_len as usize,
            )
            .unwrap();

            println!("find de name{:?}", name);

            if is_goal {
                file.inode = dir_search_result.dentry.inode;
                return;
            } else {
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

    pub fn ext4_file_write(&self, ext4_file: &mut Ext4File, data: &[u8], size: usize) {}
}

pub fn ext4_fs_put_inode_ref(inode_ref: &mut Ext4InodeRef) {
    inode_ref.inner.write_back_inode();
}

pub fn ext4_link() -> usize {
    0
}

pub fn ext4_fs_inode_blocks_init(inode_ref: &mut Ext4InodeRef) {}

pub fn ext4_fs_alloc_inode(child_inode_ref: &mut Ext4InodeRef, filetype: u8) -> usize {
    let mut is_dir = false;

    let inode_size = child_inode_ref.fs().super_block.inode_size();

    is_dir = filetype == DirEntryType::EXT4_DE_DIR.bits();

    let mut index = 0;
    let rc = ext4_ialloc_alloc_inode(child_inode_ref.fs(), &mut index, is_dir);

    0
}
pub fn ext4_dir_destroy_result() {}

pub fn ext4_dir_find_entry(
    parent: &mut Ext4InodeRef,
    name: &str,
    name_len: u32,
    result: &mut Ext4DirSearchResult,
) -> usize {
    println!("ext4_dir_find_entry {:?}", name);
    let mut iblock = 0;
    let mut fblock: ext4_fsblk_t = 0;

    let inode_size: u32 = parent.inner.inode.size;
    let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

    while iblock < total_blocks {
        ext4_fs_get_inode_dblk_idx(parent, iblock, &mut fblock, false);

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

        let r = ext4_dir_find_in_block(&mut ext4_block, name_len, name, result);

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
    let inode = &mut inode_ref.inner.inode;

    let mut vec_extent_path: Vec<Ext4ExtentPath> = Vec::with_capacity(3);

    let mut extent_path = Ext4ExtentPath::default();

    ext4_find_extent(inode, iblock, &mut extent_path, &mut vec_extent_path);

    let depth = unsafe { *ext4_inode_hdr(inode) }.depth;

    let ex: Ext4Extent = unsafe { *vec_extent_path[depth as usize].extent };

    let ee_block = ex.first_block;
    let ee_start = ex.start_lo | (((ex.start_hi as u32) << 31) << 1);
    let ee_len: u16 = ex.block_count;

    if iblock >= ee_block && iblock <= (ee_block + ee_len as u32) {
        let newblock = iblock - ee_block + ee_start;
        *result = newblock as u64;
        return;
    }
}

pub fn ext4_find_all_extent(inode_ref: &Ext4InodeRef, extents: &mut Vec<Ext4Extent>) {
    let extent_header = Ext4ExtentHeader::try_from(&inode_ref.inner.inode.block[..2]).unwrap();
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

pub fn ext4_find_extent(
    inode: &Ext4Inode,
    iblock: ext4_lblk_t,
    orig_path: &mut Ext4ExtentPath,
    v: &mut Vec<Ext4ExtentPath>,
) {
    let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;

    // println!("ext4_find_extent header {:x?}", unsafe{*eh});
    let extent_header = Ext4ExtentHeader::try_from(&inode.block[..]).unwrap();

    let depth = extent_header.depth;

    let mut extent_path = Ext4ExtentPath::default();
    extent_path.depth = depth;
    extent_path.header = eh;

    // depth = 0
    let r = ext4_ext_binsearch(&mut extent_path, iblock);
    // println!("ext4_find_extent r {:x?}", r);

    let extent = unsafe { *extent_path.extent };
    let pblock = extent.start_lo | (((extent.start_hi as u32) << 31) << 1);
    extent_path.p_block = pblock;

    // println!("ext4_find_extent extent {:x?}", extent);
    v.push(extent_path);
}

pub fn ext4_fs_get_inode_dblk_idx(
    inode_ref: &mut Ext4InodeRef,
    iblock: ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
    extent_create: bool,
) {
    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;

    let mut blocks_count = 0;
    ext4_extent_get_blocks(
        inode_ref,
        iblock,
        1,
        &mut current_fsblk,
        false,
        &mut blocks_count,
    );

    current_block = current_fsblk;
    *fblock = current_block;
}

pub fn ext4_dir_find_in_block(
    block: &Ext4Block,
    name_len: u32,
    name: &str,
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

        if let Ok(s) = s {
            if name_len == de.name_len as u32 {
                if name.to_string() == s {
                    // println!(
                    //     "found s {:?}  name_len {:x?} de.name_len {:x?}",
                    //     s, name_len, de.name_len
                    // );
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
        let super_block = fs.super_block.clone();

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

            let bitmap_data = &mut raw_data[..bitmap_size as usize];

            let mut idx_in_bg = 0 as u32;

            ext4_bmap_bit_find_clr(bitmap_data, 0, inodes_in_bg, &mut idx_in_bg);
            ext4_bmap_bit_set(&mut raw_data, idx_in_bg);

            bg.set_block_group_ialloc_bitmap_csum(&super_block, &raw_data);

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
            let mut s = super_block.clone();
            s.decrease_free_inodes_count();
            s.sync_super_block_to_disk(block_device.clone());

            /* Compute the absolute i-nodex number */
            let inodes_per_group = s.inodes_per_group();
            let inode_num = bgid * inodes_per_group + (idx_in_bg + 1);
            *index = inode_num;

            return;
        }

        bgid += 1;
    }
    println!("no free inode");
}
