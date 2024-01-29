extern crate alloc;

use crate::BLOCK_SIZE;
use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;

use crate::defs::*;

// A function that takes a &str and returns a &[char]
pub fn get_name(name: [u8; 255], len: usize) -> Result<String, string::FromUtf8Error> {
    let mut v: Vec<u8> = Vec::new();
    for i in 0..len {
        v.push(name[i]);
    }
    let s = String::from_utf8(v);
    s
}

// 打印目录项的名称和类型
pub fn print_dir_entry(entry: &Ext4DirEntry) {
    let name = str::from_utf8(&entry.name[..entry.name_len as usize]).unwrap();
    let file_type = DirEntryType::from_bits(entry.file_type).unwrap();
    match file_type {
        DirEntryType::REG_FILE => println!("{}: regular file", name),
        DirEntryType::DIR => println!("{}: directory", name),
        DirEntryType::CHRDEV => println!("{}: character device", name),
        DirEntryType::BLKDEV => println!("{}: block device", name),
        DirEntryType::FIFO => println!("{}: fifo", name),
        DirEntryType::SOCK => println!("{}: socket", name),
        DirEntryType::SYMLINK => println!("{}: symbolic link", name),
        _ => println!("{}: unknown type", name),
    }
}

pub fn read_super_block<A: Ext4Traits>() -> Ext4SuperBlock {
    let data = A::read_block(BASE_OFFSET);
    let mut buf = [0u8; size_of::<Ext4SuperBlock>()];
    buf.copy_from_slice(&data[..size_of::<Ext4SuperBlock>()]);
    unsafe { core::ptr::read(buf.as_ptr() as *const _) }
}

pub fn ext4_add_extent<A: Ext4Traits>(
    inode: &Ext4Inode,
    depth: u16,
    data: &[u32],
    extents: &mut Vec<Ext4Extent>,
    first_level: bool,
) {
    let extent_header = Ext4ExtentHeader::from_bytes_u32(data);
    let extent_entries = extent_header.eh_entries;

    println!("header {:x?}", extent_header);

    if depth == 0 {
        for en in 0..extent_entries {
            let idx = (3 + en * 3) as usize;
            let extent = Ext4Extent::from_bytes_u32(&data[idx..]);
            let ee_block = extent.first_block;
            let ee_len = extent.ee_len;
            let ee_start_hi = extent.ee_start_hi;
            let ee_start_lo = extent.ee_start_lo;
            extents.push(extent)
        }

        return;
    }

    for en in 0..extent_entries {
        let idx = (3 + en * 3) as usize;
        if idx == 12 {
            break;
        }
        let extent_index = Ext4ExtentIndex::from_bytes_u32(&data[idx..]);
        println!("extent_index {:x?}", extent_index);
        let ei_leaf_lo = extent_index.ei_leaf_lo;
        let ei_leaf_hi = extent_index.ei_leaf_hi;
        let mut block = ei_leaf_lo;
        block |= ((ei_leaf_hi as u32) << 31) << 1;
        let data = A::read_block(block as u64 * BLOCK_SIZE);
        let data: Vec<u32> = unsafe { core::mem::transmute(data) };
        ext4_add_extent::<A>(inode, depth - 1, &data, extents, false);
    }
}

pub fn ext4_path_check(path: &str, is_goal: &mut bool) -> usize {
    println!("path_check {:?}", path);
    for (i, c) in path.chars().enumerate() {
        if c == '/' {
            *is_goal = false;
            return i;
        }
    }
    let path = path.to_string();
    *is_goal = true;
    return path.len();
}

pub fn ext4_get_block_group<A: Ext4Traits>(block_group: u64, super_block: &Ext4SuperBlock) -> u64 {
    let block_size = BLOCK_SIZE;
    let dsc_cnt = block_size / super_block.desc_size as u64;
    let dsc_per_block = dsc_cnt;
    let dsc_id = block_group / dsc_cnt;
    let first_meta_bg = super_block.first_meta_bg;
    let first_data_block = super_block.first_data_block;

    let block_id = first_data_block as u64 + dsc_id + 1;

    let offset = (block_group % dsc_cnt) * super_block.desc_size as u64;

    // 读取组描述符表的数据块的内容
    let gd_block_data = A::read_block(block_id as u64 * BLOCK_SIZE);
    let gd_data = &gd_block_data[offset as usize..offset as usize + size_of::<GroupDesc>()];

    let mut gd = GroupDesc::default();

    let ptr = &mut gd as *mut GroupDesc as *mut u8;

    unsafe {
        core::ptr::copy_nonoverlapping(gd_data.as_ptr(), ptr, core::mem::size_of::<GroupDesc>());
    }

    let inode_table_blk_num = ((gd.bg_inode_table_hi as u64) << 32) | gd.bg_inode_table_lo as u64;

    return inode_table_blk_num;
}

// 从文件中读取inode
pub fn read_inode<A: Ext4Traits>(inode: u64, super_block: &Ext4SuperBlock) -> Ext4Inode {
    // println!("read inode {:x?}", inode);
    let inodes_per_group = super_block.inodes_per_group;
    let inode_size = super_block.inode_size as u64;
    let group = (inode - 1) / inodes_per_group as u64;
    let index = (inode - 1) % inodes_per_group as u64;
    let inode_table_blk_num = ext4_get_block_group::<A>(group, super_block);
    let offset = inode_table_blk_num * BLOCK_SIZE + index * inode_size;

    let data = A::read_block(offset);
    let mut buf = [0u8; 0x100];
    buf.copy_from_slice(&data[..0x100]);
    unsafe { core::ptr::read(buf.as_ptr() as *const _) }
}

// 从文件中读取inode
pub fn get_inode_block<A: Ext4Traits>(inode: u64, super_block: &Ext4SuperBlock) -> u64 {
    let inodes_per_group = super_block.inodes_per_group;
    let inode_size = super_block.inode_size as u64;
    let group = (inode - 1) / inodes_per_group as u64;
    let index = (inode - 1) % inodes_per_group as u64;

    let mut inode_table_blk_num = ext4_get_block_group::<A>(group, super_block);

    let mut offset = inode_table_blk_num * BLOCK_SIZE + index * inode_size;

    offset
}

// 从文件中读取目录项
pub fn read_dir_entry<A: Ext4Traits>(inode: u64, super_block: &Ext4SuperBlock) -> Vec<Ext4DirEntry> {
    // 调用get_inode函数，根据inode编号，获取inode的内容，存入一个Inode类型的结构体中
    let inode_data = read_inode::<A>(inode, super_block);

    let mut extents: Vec<Ext4Extent> = Vec::new();

    // 调用ext4_find_extent函数，根据inode的内容，获取inode的数据块的范围，存入一个Extent类型的向量中
    ext4_find_extent::<A>(&inode_data, &mut extents);

    // 创建一个空的DirEntry类型的向量entries，用来存放目录的目录项
    let mut entries = Vec::<Ext4DirEntry>::new();

    for e in extents {
        let blk_no: u64 = ((e.ee_start_hi as u64) << 32) | e.ee_start_lo as u64;
        for i in 0..e.ee_len {
            let block = A::read_block((blk_no + i as u64) * BLOCK_SIZE);
            let mut offset = 0;

            let mut names: Vec<Result<String, string::FromUtf8Error>>;
            while offset < block.len() {
                let de = Ext4DirEntry::from_bytes_offset(&block, offset);
                offset = offset + de.rec_len as usize;
                if de.inode == 0 {
                    continue;
                }
                // let s = get_name(de.name, de.name_len as usize);
                // print_dir_entry(&de);
                entries.push(de);
            }
        }
    }
    entries
}

pub fn ext4_find_extent<A: Ext4Traits>(inode: &Ext4Inode, extents: &mut Vec<Ext4Extent>) {
    let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode.block[..2]);

    let data = &inode.block;

    println!("inode block data {:x?}", data);

    let depth = extent_header.eh_depth;

    ext4_add_extent::<A>(inode, depth, data, extents, true);
}

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
        let ext_count = hdr_ref.eh_entries as usize;
        (hdr as *const u8).add(hdr_size + (ext_count - 1) * ext_size) as *const Ext4Extent
    }
}

pub fn ext4_last_extent_mut(hdr: *mut Ext4ExtentHeader) -> *mut Ext4Extent {
    unsafe {
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        let ext_size = core::mem::size_of::<Ext4Extent>();
        let hdr_ref = core::mem::transmute::<*mut Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        let ext_count = hdr_ref.eh_entries as usize;

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
        let ext_count = hdr_ref.eh_entries as usize;
        (hdr as *const u8).add(hdr_size + (ext_count - 1) * ext_size) as *const Ext4ExtentIndex
    }
}

pub fn ext4_last_extent_index_mut(hdr: *mut Ext4ExtentHeader) -> *mut Ext4ExtentIndex {
    unsafe {
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        let ext_size = core::mem::size_of::<Ext4ExtentIndex>();
        let hdr_ref = core::mem::transmute::<*mut Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        let ext_count = hdr_ref.eh_entries as usize;
        (hdr as *mut u8).add(hdr_size + (ext_count - 1) * ext_size) as *mut Ext4ExtentIndex
    }
}

// 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
pub fn ext4_ext_binsearch(path: &mut Ext4ExtentPath, block: ext4_lblk_t) -> bool {
    // 获取extent header的引用
    let eh = unsafe { &*path.header };

    if eh.eh_entries == 0 {
        /*
         * this leaf is empty:
         * we get such a leaf in split/add case
         */
        false;
    }

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent(eh).add(1) };
    let mut r = unsafe { ext4_last_extent(eh) };

    // 如果extent header中没有有效的entry，直接返回false
    if eh.eh_entries == 0 {
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

// 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
pub fn ext4_ext_binsearch_foo(path: &mut ext4_extent_path, block: ext4_lblk_t) -> bool {
    // 获取extent header的引用
    // let eh = unsafe { &*path.header };

    let eh = path.header;

    unsafe {
        if (*eh).eh_entries == 0 {
            return false;
        }
    }

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent_mut(eh).add(1) };
    let mut r = unsafe { ext4_last_extent_mut(eh) };

    // 如果extent header中没有有效的entry，直接返回false

    unsafe {
        if (*eh).eh_entries == 0 {
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

pub fn ext4_find_extent_new(
    inode: &Ext4Inode,
    iblock: ext4_lblk_t,
    orig_path: &mut Ext4ExtentPath,
    v: &mut Vec<Ext4ExtentPath>,
) {
    let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;

    let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode.block[..2]);
    let depth = extent_header.eh_depth;

    let mut extent_path = Ext4ExtentPath::default();
    extent_path.depth = depth;
    extent_path.header = eh;
    extent_path.block = Ext4Block::default();

    // depth = 0
    ext4_ext_binsearch(&mut extent_path, iblock);

    let extent = unsafe { *extent_path.extent };
    let pblock = extent.ee_start_lo | (((extent.ee_start_hi as u32) << 31) << 1);
    extent_path.p_block = pblock as u64;

    v.push(extent_path);
}

pub fn ext_inode_hdr(inode: &Ext4Inode) -> *const Ext4ExtentHeader {
    let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;
    eh
}

pub fn ext_inode_hdr_mut(inode: &mut Ext4Inode) -> *mut Ext4ExtentHeader {
    let eh = &mut inode.block as *mut [u32; 15] as *mut Ext4ExtentHeader;
    eh
}

pub fn ext4_extent_get_blocks(
    inode: &Ext4Inode,
    iblock: ext4_lblk_t,
    max_blocks: u32,
    result: &mut ext4_fsblk_t,
    extent_create: bool,
) {
    let mut vec_extent_path: Vec<Ext4ExtentPath> = Vec::with_capacity(3);

    let mut extent_path = Ext4ExtentPath::default();

    ext4_find_extent_new(inode, iblock, &mut extent_path, &mut vec_extent_path);

    let depth = unsafe { *ext_inode_hdr(inode) }.eh_depth;

    let ex: Ext4Extent = unsafe { *vec_extent_path[depth as usize].extent };

    let ee_block = ex.first_block;
    let ee_start = ex.ee_start_lo | (((ex.ee_start_hi as u32) << 31) << 1);
    let ee_len = ex.ee_len;

    if iblock >= ee_block && iblock <= (ee_block + ee_len as u32) {
        let newblock = iblock - ee_block + ee_start;
        *result = newblock as u64;

        return;
    }
}

pub fn ext4_fs_get_inode_dblk_idx(
    inode_ref: &Ext4Inode,
    iblock: ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
    extent_create: bool,
) {
    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;

    ext4_extent_get_blocks(inode_ref, iblock, 1, &mut current_fsblk, false);

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

    while offset < block.data.len() {
        let de = Ext4DirEntry::from_bytes_offset(&block.data, offset);

        offset = offset + de.rec_len as usize;
        if de.inode == 0 {
            continue;
        }
        let s = get_name(de.name, de.name_len as usize);

        if let Ok(s) = s {
            // println!("de {:x?}", de);
            if name_len == de.name_len as u32 {
                if name.to_string() == s {
                    println!(
                        "found s {:?}  name_len {:x?} de.name_len {:x?}",
                        s, name_len, de.name_len
                    );
                    result.dentry.entry_len = de.rec_len;
                    result.dentry.name = de.name;
                    result.dentry.name_len = de.name_len;
                    result.dentry.name_length_high = de.file_type;
                    result.dentry.inode = de.inode;

                    return true;
                }
            }
        }
    }

    false
}

pub fn ext4_dir_find_entry<A: Ext4Traits>(
    parent: &Ext4Inode,
    name: &str,
    name_len: u32,
    result: &mut Ext4DirSearchResult,
) {
    println!("ext4_dir_find_entry {:?}", name);
    let mut iblock: u32;
    let mut fblock: ext4_fsblk_t = 0;

    let inode_size: u32 = parent.size;
    let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

    /* Walk through all data blocks */
    iblock = 0;

    while iblock < total_blocks {
        ext4_fs_get_inode_dblk_idx(parent, iblock, &mut fblock, false);

        let mut b = Ext4Block::default();

        let data = A::read_block(fblock * BLOCK_SIZE);

        b.lb_id = iblock as u64;
        b.data = data;

        let r = ext4_dir_find_in_block(&mut b, name_len, name, result);

        if r {
            return;
        }

        iblock += 1;
    }

    // return ENOENT;
}

pub fn ext4_path_skip_dot(path: &str) -> &str {
    let path_skip_dot = path.trim_start_matches(".");

    // let path_skip_slash = path_skip_dot.trim_start_matches("/");

    path_skip_dot
}

pub fn ext4_path_skip_slash(path: &str) -> &str {
    let path_skip_slash = path.trim_start_matches("/");

    path_skip_slash
}

pub fn ext4_generic_open<A: Ext4Traits>(ext4_file: &mut Ext4File, path: &str) {
    let mp = &ext4_file.mp;

    let super_block = read_super_block::<A>();

    let mut dir_search_result = Ext4DirSearchResult::default();

    let mut is_goal = false;

    // start from root
    dir_search_result.dentry.inode = 2;

    let mut search_path = ext4_path_skip_dot(&path);

    let mut len = 0;
    loop {
        search_path = ext4_path_skip_slash(search_path);
        // println!("search path {:?}", search_path);
        len = ext4_path_check(search_path, &mut is_goal);

        let inode_data = read_inode::<A>(dir_search_result.dentry.inode as u64, &super_block);

        ext4_dir_find_entry::<A>(
            &inode_data,
            &search_path[..len],
            len as u32,
            &mut dir_search_result,
        );

        let name = get_name(
            dir_search_result.dentry.name,
            dir_search_result.dentry.name_len as usize,
        )
        .unwrap();

        // println!("name {:?}", name);

        if is_goal {
            ext4_file.inode = dir_search_result.dentry.inode;
            return;
        } else {
            search_path = &search_path[len..];
        }
    }

    // // final dir
    // let inode_data = read_inode::<A>( dir_search_result.dentry.inode as u64, &super_block);
    // ext4_dir_find_entry::<A>( &inode_data, &search_path[..len], len as u32, &mut dir_search_result);
    // let name = get_name(dir_search_result.dentry.name, dir_search_result.dentry.name_len as usize).unwrap();
    // println!("name {:?}", name);
    // println!("file inode num {:?}", dir_search_result.dentry.inode);
    // ext4_file.inode = dir_search_result.dentry.inode;
}

pub fn ext4_file_read<A: Ext4Traits>(ext4_file: &mut Ext4File) {
    let super_block = read_super_block::<A>();
    let inode_data = read_inode::<A>(ext4_file.inode as u64, &super_block);
    // let mut extents:Vec<Ext4Extent> = Vec::new();
    // ext4_find_extent(file, &inode_data, &mut extents);

    let size = inode_data.size as usize;

    // 创建一个空的向量，用于存储文件的内容
    let mut file_data: Vec<u8> = Vec::new();

    // 创建一个空的向量，用于存储文件的所有extent信息
    let mut extents: Vec<Ext4Extent> = Vec::new();

    // 从inode_data中获取文件的所有extent信息，并存储在extents向量中
    ext4_find_extent::<A>(&inode_data, &mut extents);

    // println!("extents {:x?}", extents);
    // 遍历extents向量，对每个extent，计算它的物理块号，然后调用read_block函数来读取数据块，并将结果追加到file_data向量中
    for extent in extents {
        // 获取extent的起始块号、块数和逻辑块号
        let start_block = extent.ee_start_lo as u64 | ((extent.ee_start_hi as u64) << 32);
        let block_count = extent.ee_len as u64;
        let logical_block = extent.first_block as u64;

        // 计算extent的物理块号
        let physical_block = start_block + logical_block;

        // 从file中读取extent的所有数据块，并将结果追加到file_data向量中
        for i in 0..block_count {
            let block_num = physical_block + i;
            // println!("read block num {:x?}", block_num);
            let block_data = A::read_block(block_num * BLOCK_SIZE);

            file_data.extend(block_data);
        }
    }

    // println!("file_data  {:x?}", &file_data[..10]);

    // println!("size {:x?}", size);
}

pub fn ext4_file_read_foo<A: Ext4Traits>(ext4_file: &mut Ext4File) -> Vec<u8> {
    let super_block = read_super_block::<A>();
    let inode_data = read_inode::<A>(ext4_file.inode as u64, &super_block);

    // let mut extents:Vec<Ext4Extent> = Vec::new();
    // ext4_find_extent(file, &inode_data, &mut extents);

    let size: usize = inode_data.size as usize;

    println!("inode num {:x?} size {:x?}", ext4_file.inode, size);
    // 创建一个空的向量，用于存储文件的内容
    let mut file_data: Vec<u8> = Vec::new();

    // 创建一个空的向量，用于存储文件的所有extent信息
    let mut extents: Vec<Ext4Extent> = Vec::new();

    // 从inode_data中获取文件的所有extent信息，并存储在extents向量中
    ext4_find_extent::<A>(&inode_data, &mut extents);

    println!("extents {:x?}", extents);

    // 遍历extents向量，对每个extent，计算它的物理块号，然后调用read_block函数来读取数据块，并将结果追加到file_data向量中
    for extent in extents {
        // 获取extent的起始块号、块数和逻辑块号
        let start_block = extent.ee_start_lo as u64 | ((extent.ee_start_hi as u64) << 32);
        let block_count = extent.ee_len as u64;
        let logical_block = extent.first_block as u64;

        // 计算extent的物理块号
        let physical_block = start_block + logical_block;

        // 从file中读取extent的所有数据块，并将结果追加到file_data向量中
        for i in 0..block_count {
            let block_num = physical_block + i;
            let block_data = A::read_block(block_num * BLOCK_SIZE);
            println!("read block num {:x?}", block_num);

            file_data.extend(block_data);
        }
    }

    println!("file_data  {:x?}", &file_data[..10]);
    file_data

    // println!("size {:x?}", size);
}

pub fn ext4_parse_flags(flags: Option<&str>, file_flags: &mut u32) -> bool {
    match flags {
        None => false,
        Some("r") | Some("rb") => {
            *file_flags = O_RDONLY;
            true
        }
        Some("w") | Some("wb") => {
            *file_flags = O_WRONLY | O_CREAT | O_TRUNC;
            true
        }
        Some("a") | Some("ab") => {
            *file_flags = O_WRONLY | O_CREAT | O_APPEND;
            true
        }
        Some("r+") | Some("rb+") | Some("r+b") => {
            *file_flags = O_RDWR;
            true
        }
        Some("w+") | Some("wb+") | Some("w+b") => {
            *file_flags = O_RDWR | O_CREAT | O_TRUNC;
            true
        }
        Some("a+") | Some("ab+") | Some("a+b") => {
            *file_flags = O_RDWR | O_CREAT | O_APPEND;
            true
        }
        _ => false,
    }
}

#[derive(Debug)]
pub struct Ext4Fs<A: Ext4Traits> {
    superblock: Ext4SuperBlock,
    inode_block_limits: [u64; 4],
    inode_blocks_per_level: [u64; 4],
    last_inode_bg_id: u32,
    phatom: PhantomData<A>,
}

impl<A: Ext4Traits> Ext4Fs<A> {
    pub fn init() -> Self {
        let super_block = read_super_block::<A>();
        let block_size: u32 = super_block.log_block_size;

        let mut inode_block_limits = [0u64; 4];
        let mut inode_blocks_per_level = [0u64; 4];

        inode_block_limits[0] = EXT4_INODE_DIRECT_BLOCK_COUNT as u64;
        inode_blocks_per_level[0] = 1;

        let blocks_id = (1024 << block_size) as usize / core::mem::size_of::<u32>();
        for i in 1..4 {
            inode_blocks_per_level[i] = inode_blocks_per_level[i - 1] * blocks_id as u64;
            inode_block_limits[i] = inode_block_limits[i - 1] + inode_blocks_per_level[i];
        }

        Self {
            superblock: super_block,
            inode_block_limits: inode_block_limits,
            inode_blocks_per_level: inode_blocks_per_level,
            last_inode_bg_id: 0,
            phatom: PhantomData,
        }
    }
}

pub fn ext4_fs_get_block_group_ref<A: Ext4Traits>(block_group: u64) -> *const GroupDesc {
    let super_block = read_super_block::<A>();
    let block_size = BLOCK_SIZE;
    let dsc_cnt = block_size / super_block.desc_size as u64;
    let dsc_per_block = dsc_cnt;
    let dsc_id = block_group / dsc_cnt;
    let first_meta_bg = super_block.first_meta_bg;
    let first_data_block = super_block.first_data_block;

    let block_id = first_data_block as u64 + dsc_id + 1;

    let offset = (block_group % dsc_cnt) * super_block.desc_size as u64;

    let gd_block_data = A::read_block(block_id as u64 * BLOCK_SIZE);
    let gd_data = &gd_block_data[offset as usize..offset as usize + size_of::<GroupDesc>()];

    let mut gd = GroupDesc::default();

    let ptr = &mut gd as *mut GroupDesc as *mut u8;

    unsafe {
        core::ptr::copy_nonoverlapping(gd_data.as_ptr(), ptr, core::mem::size_of::<GroupDesc>());
    }

    ptr as _
}

pub fn ext4_block_group_des_write_back<A: Ext4Traits>(block_group: u64, gd: GroupDesc) {
    let super_block = read_super_block::<A>();
    let block_size = BLOCK_SIZE;
    let dsc_cnt = block_size / super_block.desc_size as u64;
    let dsc_id = block_group / dsc_cnt;
    let first_data_block = super_block.first_data_block;

    let block_id = first_data_block as u64 + dsc_id + 1;

    let offset = (block_group % dsc_cnt) * super_block.desc_size as u64;

    // 读取组描述符表的数据块的内容
    let mut gd_block_data = A::read_block(block_id as u64 * BLOCK_SIZE);
    // let gd_data = &gd_block_data[offset as usize..offset as usize + size_of::<GroupDesc>()];
    copy_block_group_to_array(
        &gd,
        &mut gd_block_data[offset as usize..offset as usize + size_of::<GroupDesc>()],
        0,
    );
    A::write_block(block_id as u64 * BLOCK_SIZE, &gd_block_data);
}

pub fn copy_block_group_to_array(gd: &GroupDesc, array: &mut [u8], offset: usize) {
    // 使用unsafe代码块，因为涉及到裸指针和类型转换
    unsafe {
        // 把header的引用转换为一个u32的指针
        let gd_ptr = gd as *const GroupDesc as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u8] as *mut u8;

        let count = core::mem::size_of::<GroupDesc>();
        core::ptr::copy_nonoverlapping(gd_ptr, array_ptr.add(offset), count);
    }
}

pub fn copy_super_block_to_array(super_block: &Ext4SuperBlock, array: &mut [u8]) {
    // 使用unsafe代码块，因为涉及到裸指针和类型转换
    unsafe {
        // 把header的引用转换为一个u32的指针
        let super_block_ptr = super_block as *const Ext4SuperBlock as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u8] as *mut u8;

        let count = core::mem::size_of::<Ext4SuperBlock>();
        core::ptr::copy_nonoverlapping(super_block_ptr, array_ptr, count);
    }
}

pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;

pub fn ext4_sb_get_desc_size<A: Ext4Traits>() -> u16 {
    let s = read_super_block::<A>();

    let size = s.desc_size;

    // println!("desc size {:x?}", size);
    if size < EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        return EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE as u16;
    } else {
        size
    }
}

/// 设置块组中的空闲i节点数
/// bg: 块组的指针
/// s: 超级块的指针
/// cnt: 块组中的空闲i节点数
pub fn ext4_bg_set_free_inodes_count<A: Ext4Traits>(gd: &mut GroupDesc, cnt: u32) {
    gd.bg_free_inodes_count_lo = ((cnt << 16) >> 16) as u16;
    if ext4_sb_get_desc_size::<A>() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        gd.bg_free_inodes_count_hi = (cnt >> 16) as u16;
    }
}

pub fn ext4_bg_get_free_inodes_count<A: Ext4Traits>(gd: &GroupDesc) -> u32 {
    let mut free_inodes = gd.bg_free_inodes_count_lo as u32;

    let s = read_super_block::<A>();

    if ext4_sb_get_desc_size::<A>() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        free_inodes |= (gd.bg_free_inodes_count_hi as u32) << 16;
    }

    free_inodes
}

pub fn ext4_bg_get_used_dirs_count<A: Ext4Traits>(gd: &GroupDesc) -> u32 {
    let mut used_dirs = gd.bg_used_dirs_count_lo as u32;

    let s = read_super_block::<A>();

    if ext4_sb_get_desc_size::<A>() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        used_dirs |= (gd.bg_used_dirs_count_hi as u32) << 16;
    }

    used_dirs
}

/// 获取包含i节点位图的块的地址。
/// @param bg 块组的指针
/// @param s 超级块的指针
/// @return 包含i节点位图的块的地址
pub fn ext4_bg_get_inode_bitmap<A: Ext4Traits>(bg: &GroupDesc) -> u64 {
    let s = read_super_block::<A>();

    let mut v = u32::from_le(bg.bg_inode_bitmap_lo) as u64;

    let desc_size = ext4_sb_get_desc_size::<A>();
    if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        v |= (u32::from_le(bg.bg_inode_bitmap_hi) as u64) << 32;
    }

    v
}

/**@brief Get number of unused i-nodes.
 * @param bg Pointer to block group
 * @param s Pointer to superblock
 * @return Number of unused i-nodes
 */
pub fn ext4_bg_get_itable_unused<A: Ext4Traits>(bg: &GroupDesc) -> u16 {
    let mut v = bg.bg_itable_unused_lo;

    let desc_size = ext4_sb_get_desc_size::<A>();

    if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        v |= ((bg.bg_itable_unused_hi as u64) << 32) as u16;
    }

    return v;
}

/**@brief Set number of unused i-nodes.
 * @param bg Pointer to block group
 * @param s Pointer to superblock
 * @param cnt Number of unused i-nodes
 */
pub fn ext4_bg_set_itable_unused<A: Ext4Traits>(bg: &mut GroupDesc, cnt: u16) {
    bg.bg_itable_unused_lo = (((cnt as u32) << 16) >> 16) as u16;
    if ext4_sb_get_desc_size::<A>() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        bg.bg_itable_unused_hi = ((cnt as u32) >> 16) as u16;
    }
}

/// 获取给定块组的inode数量。
/// @param s 超级块的引用
/// @param bgid 块组的编号
/// @return 给定块组的i节点数量
pub fn ext4_inodes_in_group_cnt<A: Ext4Traits>(bgid: u32) -> u32 {
    let s = read_super_block::<A>();
    let block_group_count = ext4_block_group_cnt::<A>();
    let inodes_per_group = s.inodes_per_group;
    let total_inodes = ((s.inodes_count as u64) << 32) as u32;

    if bgid < block_group_count - 1 {
        inodes_per_group
    } else {
        total_inodes - ((block_group_count - 1) * inodes_per_group)
    }
}

pub fn ext4_block_group_cnt<A: Ext4Traits>() -> u32 {
    let s = read_super_block::<A>();
    let blocks_count: u64 = ext4_sb_get_blocks_cnt::<A>();
    let blocks_per_group: u32 = s.blocks_per_group;
    let mut block_groups_count: u32 = (blocks_count / blocks_per_group as u64) as u32;

    if blocks_count % blocks_per_group as u64 != 0 {
        block_groups_count += 1;
    }
    block_groups_count
}

pub fn ext4_ialloc_bgidx_to_inode(index: u32, bgid: u32, super_block: &Ext4SuperBlock) -> u32 {
    let inodes_per_group = super_block.inodes_per_group;

    bgid * inodes_per_group + (index + 1)
}

/// Blocks count get stored in superblock.
///
/// # Arguments
///
/// * `s` - superblock descriptor
///
/// # Returns
///
/// Count of blocks
pub fn ext4_sb_get_blocks_cnt<A: Ext4Traits>() -> u64 {
    let s = read_super_block::<A>();
    ((s.blocks_count_hi.to_le() as u64) << 32) | s.blocks_count.to_le() as u64
}

// /// 检查位图中的某一位是否被设置。
// /// @param bmap 位图的指针
// /// @param bit 要检查的位
// pub fn ext4_bmap_is_bit_set(bmap: &u8, bit: u32) -> bool {
// 	(*(bmap + (bit >> 3)) & (1 << (bit & 7))) != 0
// }

/// 检查位图中的某一位是否被设置
/// 参数 bmap 位图缓冲区
/// 参数 bit 要检查的位
pub fn ext4_bmap_is_bit_set(bmap: &[u8], bit: u32) -> bool {
    // 使用位运算和数组索引来访问位图中的对应位
    bmap[(bit >> 3) as usize] & (1 << (bit & 7)) != 0
}

/// 检查位图中的某一位是否被清除
/// 参数 bmap 位图缓冲区
/// 参数 bit 要检查的位
pub fn ext4_bmap_is_bit_clr(bmap: &[u8], bit: u32) -> bool {
    // 使用逻辑非运算符和之前定义的函数来判断位是否被清除
    !ext4_bmap_is_bit_set(bmap, bit)
}

/// 设置位图中的某一位
/// 参数 bmap 位图
/// 参数 bit 要设置的位
pub fn ext4_bmap_bit_set(bmap: &mut [u8], bit: u32) {
    bmap[(bit >> 3) as usize] |= 1 << (bit & 7);
}

/// 计算CRC32校验和
/// 参数 crc 初始值
/// 参数 buf 缓冲区
/// 参数 size 缓冲区大小
/// 参数 tab 查找表
pub fn crc32(crc: u32, buf: &[u8], size: u32, tab: &[u32]) -> u32 {
    let mut crc = crc;
    let mut p = buf;
    let mut size = size as usize;

    // println!("crc32 buf size={:x?}", size);
    // println!("buf {:x?}", &buf);
    // 循环更新crc值
    while size > 0 {
        // 使用异或运算和查找表来计算crc
        crc = tab[(crc as u8 ^ p[0]) as usize] ^ (crc >> 8);
        // println!("crc {:x?}", crc);
        // 移动缓冲区指针
        p = &p[1..];
        // 减少剩余大小
        size -= 1;
    }

    crc
}

/* */
/* CRC LOOKUP TABLE */
/* ================ */
/* The following CRC lookup table was generated automagically */
/* by the Rocksoft^tm Model CRC Algorithm Table Generation */
/* Program V1.0 using the following model parameters: */
/* */
/* Width : 4 bytes. */
/* Poly : 0x1EDC6F41L */
/* Reverse : TRUE. */
/* */
/* For more information on the Rocksoft^tm Model CRC Algorithm, */
/* see the document titled "A Painless Guide to CRC Error */
/* Detection Algorithms" by Ross Williams */
/* (ross@guest.adelaide.edu.au.). This document is likely to be */
/* in the FTP archive "ftp.adelaide.edu.au/pub/rocksoft". */
/* */
pub const CRC32C_TAB: [u32; 256] = [
    0x00000000, 0xF26B8303, 0xE13B70F7, 0x1350F3F4, 0xC79A971F, 0x35F1141C, 0x26A1E7E8, 0xD4CA64EB,
    0x8AD958CF, 0x78B2DBCC, 0x6BE22838, 0x9989AB3B, 0x4D43CFD0, 0xBF284CD3, 0xAC78BF27, 0x5E133C24,
    0x105EC76F, 0xE235446C, 0xF165B798, 0x030E349B, 0xD7C45070, 0x25AFD373, 0x36FF2087, 0xC494A384,
    0x9A879FA0, 0x68EC1CA3, 0x7BBCEF57, 0x89D76C54, 0x5D1D08BF, 0xAF768BBC, 0xBC267848, 0x4E4DFB4B,
    0x20BD8EDE, 0xD2D60DDD, 0xC186FE29, 0x33ED7D2A, 0xE72719C1, 0x154C9AC2, 0x061C6936, 0xF477EA35,
    0xAA64D611, 0x580F5512, 0x4B5FA6E6, 0xB93425E5, 0x6DFE410E, 0x9F95C20D, 0x8CC531F9, 0x7EAEB2FA,
    0x30E349B1, 0xC288CAB2, 0xD1D83946, 0x23B3BA45, 0xF779DEAE, 0x05125DAD, 0x1642AE59, 0xE4292D5A,
    0xBA3A117E, 0x4851927D, 0x5B016189, 0xA96AE28A, 0x7DA08661, 0x8FCB0562, 0x9C9BF696, 0x6EF07595,
    0x417B1DBC, 0xB3109EBF, 0xA0406D4B, 0x522BEE48, 0x86E18AA3, 0x748A09A0, 0x67DAFA54, 0x95B17957,
    0xCBA24573, 0x39C9C670, 0x2A993584, 0xD8F2B687, 0x0C38D26C, 0xFE53516F, 0xED03A29B, 0x1F682198,
    0x5125DAD3, 0xA34E59D0, 0xB01EAA24, 0x42752927, 0x96BF4DCC, 0x64D4CECF, 0x77843D3B, 0x85EFBE38,
    0xDBFC821C, 0x2997011F, 0x3AC7F2EB, 0xC8AC71E8, 0x1C661503, 0xEE0D9600, 0xFD5D65F4, 0x0F36E6F7,
    0x61C69362, 0x93AD1061, 0x80FDE395, 0x72966096, 0xA65C047D, 0x5437877E, 0x4767748A, 0xB50CF789,
    0xEB1FCBAD, 0x197448AE, 0x0A24BB5A, 0xF84F3859, 0x2C855CB2, 0xDEEEDFB1, 0xCDBE2C45, 0x3FD5AF46,
    0x7198540D, 0x83F3D70E, 0x90A324FA, 0x62C8A7F9, 0xB602C312, 0x44694011, 0x5739B3E5, 0xA55230E6,
    0xFB410CC2, 0x092A8FC1, 0x1A7A7C35, 0xE811FF36, 0x3CDB9BDD, 0xCEB018DE, 0xDDE0EB2A, 0x2F8B6829,
    0x82F63B78, 0x709DB87B, 0x63CD4B8F, 0x91A6C88C, 0x456CAC67, 0xB7072F64, 0xA457DC90, 0x563C5F93,
    0x082F63B7, 0xFA44E0B4, 0xE9141340, 0x1B7F9043, 0xCFB5F4A8, 0x3DDE77AB, 0x2E8E845F, 0xDCE5075C,
    0x92A8FC17, 0x60C37F14, 0x73938CE0, 0x81F80FE3, 0x55326B08, 0xA759E80B, 0xB4091BFF, 0x466298FC,
    0x1871A4D8, 0xEA1A27DB, 0xF94AD42F, 0x0B21572C, 0xDFEB33C7, 0x2D80B0C4, 0x3ED04330, 0xCCBBC033,
    0xA24BB5A6, 0x502036A5, 0x4370C551, 0xB11B4652, 0x65D122B9, 0x97BAA1BA, 0x84EA524E, 0x7681D14D,
    0x2892ED69, 0xDAF96E6A, 0xC9A99D9E, 0x3BC21E9D, 0xEF087A76, 0x1D63F975, 0x0E330A81, 0xFC588982,
    0xB21572C9, 0x407EF1CA, 0x532E023E, 0xA145813D, 0x758FE5D6, 0x87E466D5, 0x94B49521, 0x66DF1622,
    0x38CC2A06, 0xCAA7A905, 0xD9F75AF1, 0x2B9CD9F2, 0xFF56BD19, 0x0D3D3E1A, 0x1E6DCDEE, 0xEC064EED,
    0xC38D26C4, 0x31E6A5C7, 0x22B65633, 0xD0DDD530, 0x0417B1DB, 0xF67C32D8, 0xE52CC12C, 0x1747422F,
    0x49547E0B, 0xBB3FFD08, 0xA86F0EFC, 0x5A048DFF, 0x8ECEE914, 0x7CA56A17, 0x6FF599E3, 0x9D9E1AE0,
    0xD3D3E1AB, 0x21B862A8, 0x32E8915C, 0xC083125F, 0x144976B4, 0xE622F5B7, 0xF5720643, 0x07198540,
    0x590AB964, 0xAB613A67, 0xB831C993, 0x4A5A4A90, 0x9E902E7B, 0x6CFBAD78, 0x7FAB5E8C, 0x8DC0DD8F,
    0xE330A81A, 0x115B2B19, 0x020BD8ED, 0xF0605BEE, 0x24AA3F05, 0xD6C1BC06, 0xC5914FF2, 0x37FACCF1,
    0x69E9F0D5, 0x9B8273D6, 0x88D28022, 0x7AB90321, 0xAE7367CA, 0x5C18E4C9, 0x4F48173D, 0xBD23943E,
    0xF36E6F75, 0x0105EC76, 0x12551F82, 0xE03E9C81, 0x34F4F86A, 0xC69F7B69, 0xD5CF889D, 0x27A40B9E,
    0x79B737BA, 0x8BDCB4B9, 0x988C474D, 0x6AE7C44E, 0xBE2DA0A5, 0x4C4623A6, 0x5F16D052, 0xAD7D5351,
];

pub fn ext4_link<A: Ext4Traits>(
    mp: &Ext4MountPoint,
    parent_inode: &Ext4Inode,
    child_inode: &mut Ext4InodeRef,
    path: &str,
    len: u32,
    rename: bool,
) {
    let mut dir_search_result = Ext4DirSearchResult::default();
    ext4_dir_find_entry::<A>(&parent_inode, &path, len as u32, &mut dir_search_result);

    dir_search_result.dentry.inode = 2;

    println!("dir serach_result {:x?}", dir_search_result.dentry);

    println!("parent_inode.block{:x?}", parent_inode.block);

    let fblock = parent_inode.block[5];

    let block_data = A::read_block(fblock as u64 * BLOCK_SIZE);
    let tail_de = ext4_dir_get_tail::<A>(&block_data);
    println!("tail_de {:x?}", tail_de);

    /* Add entry to parent directory */
    ext4_dir_add_entry::<A>(parent_inode, child_inode, path, len);

    let mut b = Ext4Block::default();

    println!("-------------------read block {:x?}", fblock);

    let s = read_super_block::<A>();
    let offset = get_inode_block::<A>(2 as u64, &s);
    let mut block_data = A::read_block(offset);
}

pub fn ext4_dir_csum<A: Ext4Traits>(index: u32, dirent: &Ext4DirEntry) -> u32 {
    // println!("ext4_dir_csum de {:x?}", dirent);
    let ino_index = index;
    let ino_gen = 0 as u32;

    let mut csum = 0;

    let super_block = read_super_block::<A>();

    let uuid = super_block.uuid;

    csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
    csum = ext4_crc32c(csum, &ino_index.to_le_bytes(), 4);
    csum = ext4_crc32c(csum, &ino_gen.to_le_bytes(), 4);

    let mut data = [0u8; 0xff4];

    copy_diren_to_array(&dirent, &mut data);

    // println!("dir data {:x?}", &data);
    // println!("csum before dir {:x?}", csum);
    csum = ext4_crc32c(csum, &data, 0xff4);
    // println!("csum after dir {:x?}", csum);

    csum
}

// 用 Rust 实现 pub fn ext4_dir_csum_verify(inode: &Ext4Inode, dirent: &Ext4DirEn) {} 函数
pub fn ext4_dir_csum_verify<A: Ext4Traits>(dir_en: &mut Ext4DirEntry, block_data: &mut [u8]) -> bool {
    // 获取超级块和 UUID
    let super_block = read_super_block::<A>();
    let uuid = super_block.uuid;

    // 获取目录项的尾部
    let tail = ext4_dir_get_tail::<A>(&block_data).unwrap();

    let csum = ext4_dir_csum::<A>(dir_en.inode, &dir_en);

    if tail.checksum == csum {
        true
    } else {
        false
    }
}

pub fn ext4_dir_add_entry<A: Ext4Traits>(
    parent_inode: &Ext4Inode,
    child_inode: &mut Ext4InodeRef,
    path: &str,
    len: u32,
) {
    let s = read_super_block::<A>();

    /* Linear algorithm */
    let iblock = 0;
    let fblock = 0;
    let block_size = ext4_sb_get_block_size::<A>();
    let inode_size = ext4_inode_get_size(&s, &parent_inode);
    let total_blocks = inode_size as u32 / block_size;

    let mut iblock: u32 = 0;
    let mut fblock: ext4_fsblk_t = 0;
    let mut success = false;

    let mut extents: Vec<Ext4Extent> = Vec::new();
    ext4_find_extent::<A>(&parent_inode, &mut extents);
    let e = extents[0];
    let tmp_blk_no: u64 = ((e.ee_start_hi as u64) << 32) | e.ee_start_lo as u64;
    let mut tmp_block = A::read_block((tmp_blk_no as u64) * BLOCK_SIZE);

    let mut parent_blk = Ext4Block::default();
    let mut parent_de = Ext4DirEntry::from_bytes_offset(&tmp_block, 0);

    while iblock < total_blocks {
        ext4_fs_get_inode_dblk_idx(parent_inode, iblock, &mut fblock, false);

        // println!("iblock={:x?} fblock={:x?}", iblock, fblock);
        let mut b = Ext4Block::default();

        // println!("-------------------read block {:x?}", fblock);
        let data = A::read_block(fblock * BLOCK_SIZE);

        b.lb_id = iblock as u64;
        b.data = data;
        b.db_id = fblock;

        // println!(
        //     "ext4_dir_try_insert_entry iblock {:x?} fblock {:x?}",
        //     iblock, fblock
        // );

        let r = ext4_dir_try_insert_entry::<A>(parent_inode, &mut b, child_inode, path, len);

        // println!("ext4_dir_try_insert_entry finished ");
        A::write_block(fblock * BLOCK_SIZE, &b.data);

        let mut blk = Ext4Block::default();
        let data = A::read_block(fblock * BLOCK_SIZE);
        // blk.lb_id = 2 as u64;
        blk.data = data;
        let mut dir_search_result = Ext4DirSearchResult::default();
        dir_search_result.dentry.inode = 2;
        let r = ext4_dir_find_in_block(&mut b, 9, path, &mut dir_search_result);
        // println!("find ?{:?}", r);

        // println!("parent_de {:x?}", parent_de);
        let mut extents: Vec<Ext4Extent> = Vec::new();
        ext4_find_extent::<A>(&parent_inode, &mut extents);
        let e = extents[0];
        let tmp_blk_no: u64 = ((e.ee_start_hi as u64) << 32) | e.ee_start_lo as u64;
        let mut tmp_block = A::read_block((tmp_blk_no as u64) * BLOCK_SIZE);

        let mut parent_blk = Ext4Block::default();
        let mut parent_de = Ext4DirEntry::from_bytes_offset(&tmp_block, 0);

        let tail: Ext4DirEntryTail = ext4_dir_set_csum::<A>(&mut parent_de, &mut tmp_block);
        parent_blk.data = tmp_block;
        parent_blk.db_id = tmp_blk_no as u64;
        parent_blk.lb_id = 0;
        ext4_dir_csum_write_back::<A>(&tail, &mut parent_blk);

        if r {
            return;
        }
    }
}

pub fn ext4_dir_csum_write_back<A: Ext4Traits>(dir_en_tail: &Ext4DirEntryTail, dst_blk: &mut Ext4Block) {
    let block_size = ext4_sb_get_block_size::<A>();
    let offset = block_size as usize - core::mem::size_of::<Ext4DirEntryTail>();

    copy_diren_tail_to_array(dir_en_tail, &mut dst_blk.data, offset);
    A::write_block(dst_blk.db_id * BLOCK_SIZE, &dst_blk.data);
}

pub fn copy_diren_tail_to_array(dir_en: &Ext4DirEntryTail, array: &mut [u8], offset: usize) {
    unsafe {
        // 把header的引用转换为一个u32的指针
        let de_ptr = dir_en as *const Ext4DirEntryTail as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u8] as *mut u8;

        let count = core::mem::size_of::<Ext4DirEntryTail>();
        // println!("tail_dir_en_size {:x?}", count);
        core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
    }
}

pub fn ext4_dir_set_csum<A: Ext4Traits>(dir_en: &mut Ext4DirEntry, block_data: &mut [u8]) -> Ext4DirEntryTail {
    let mut tail = ext4_dir_get_tail::<A>(&block_data).unwrap();

    let csum = ext4_dir_csum::<A>(dir_en.inode, &dir_en);

    // println!("ext4_dir_set_csum inode {:x?} ", dir_en.inode);
    tail.checksum = csum;

    tail
}

// 尝试在一个ext4目录块中插入一个新的目录项
pub fn ext4_dir_try_insert_entry<A: Ext4Traits>(
    inode_ref: &Ext4Inode,
    dst_blk: &mut Ext4Block,
    child: &mut Ext4InodeRef,
    name: &str,
    name_len: u32,
) -> bool {
    // println!("----ext4_dir_try_insert_entry----");
    let s = read_super_block::<A>();
    // 计算新目录项所需的长度，并对齐到4字节
    let block_size = ext4_sb_get_block_size::<A>();

    let mut required_len = core::mem::size_of::<Ext4DirEntry>() + name_len as usize;

    if required_len % 4 != 0 {
        required_len += 4 - required_len % 4;
    }

    let mut offset = 0;
    while offset < dst_blk.data.len() {
        let mut de = Ext4DirEntry::from_bytes_offset(&dst_blk.data, offset);
        if de.inode == 0 {
            continue;
        }
        let inode = de.inode;
        let rec_len = de.rec_len;

        // println!("ext4_dir_try_insert_entry--de.inode {:x?}", de.inode);

        // 如果是有效的目录项，尝试分割它
        if inode != 0 {
            let used_len = de.name_len as usize;
            let mut sz = core::mem::size_of::<Ext4FakeDirEntry>() + used_len as usize;

            if used_len % 4 != 0 {
                sz += 4 - used_len % 4;
            }

            let free_space = rec_len as usize - sz;
            // println!("sz={:x?}", sz);
            // 如果有足够的空闲空间
            if free_space >= required_len {
                let mut new_entry = Ext4DirEntry::default();
                // println!("found free_space {:x?}", free_space);

                ext4_dir_write_entry(&mut new_entry, free_space as u16, &child, name, name_len);

                de.rec_len = sz as u16;

                copy_dir_entry_to_array(&de, &mut dst_blk.data, offset);
                copy_dir_entry_to_array(&new_entry, &mut dst_blk.data, offset + sz);

                let tail: Ext4DirEntryTail = ext4_dir_set_csum::<A>(&mut de, &mut dst_blk.data);
                ext4_dir_csum_write_back::<A>(&tail, dst_blk);

                break;
            }
        }

        // println!()

        offset = offset + de.rec_len as usize;
    }

    // println!("data {:x?}", &dst_blk.data[offset..]);

    return true;
}

// 写入一个ext4目录项
pub fn ext4_dir_write_entry(
    en: &mut Ext4DirEntry,
    entry_len: u16,
    child: &Ext4InodeRef,
    name: &str,
    name_len: u32,
) {
    let file_type = child.inode.mode & EXT4_INODE_MODE_TYPE_MASK;
    let file_type = DirEntryType::from_bits(file_type as u8).unwrap();

    // 设置目录项的类型
    // let name = str::from_utf8(&en.name[..en.name_len as usize]).unwrap();
    // let file_type = DirEntryType::from_bits(en.file_type).unwrap();

    match file_type {
        DirEntryType::REG_FILE => en.file_type = DirEntryType::REG_FILE.bits(),
        DirEntryType::DIR => en.file_type = DirEntryType::DIR.bits(),
        DirEntryType::CHRDEV => en.file_type = DirEntryType::CHRDEV.bits(),
        DirEntryType::BLKDEV => en.file_type = DirEntryType::BLKDEV.bits(),
        DirEntryType::FIFO => en.file_type = DirEntryType::FIFO.bits(),
        DirEntryType::SOCK => en.file_type = DirEntryType::SOCK.bits(),
        DirEntryType::SYMLINK => en.file_type = DirEntryType::SYMLINK.bits(),
        _ => println!("{}: unknown type", file_type.bits()),
    }

    en.inode = child.index;
    en.rec_len = entry_len;
    en.name_len = name_len as u8;

    let mut name_vec = [0u8; 255];

    unsafe {
        let ptr = name.as_ptr();
        let slice = core::slice::from_raw_parts(ptr, name_len as usize);
        name_vec[..name_len as usize].copy_from_slice(slice);
    }

    en.name = name_vec;

    println!("-----------en inode {:x?}", en.inode);
}

pub fn ext4_sb_get_block_size<A: Ext4Traits>() -> u32 {
    let s = read_super_block::<A>();

    1024 << s.log_block_size
}

// 获取一个ext4_inode结构体的大小
pub fn ext4_inode_get_size(s: &Ext4SuperBlock, inode: &Ext4Inode) -> u64 {
    let mut mode = inode.mode;

    // 获取inode的低32位大小
    let mut v = inode.size as u64;
    // 如果文件系统的版本号大于0，并且inode的类型是文件
    if s.rev_level > 0 && (mode & EXT4_INODE_MODE_TYPE_MASK) == EXT4_INODE_MODE_FILE {
        // 获取inode的高32位大小，并左移32位
        let hi = (inode.size_hi as u64) << 32;
        // 用或运算符将低32位和高32位拼接为一个u64值
        v |= hi;
    }
    // 返回inode的大小
    v
}

pub fn get_direntry(data: &[u8], offset: usize) -> *mut Ext4DirEntry {
    let mut header_ptr = data.as_ptr();
    unsafe {
        header_ptr.add(offset);
    }
    header_ptr as *mut Ext4DirEntry
}

pub fn ext4_inode_get_extent_header(inode_ref: &mut Ext4Inode) -> *mut Ext4ExtentHeader {
    // 使用ptr::addr_of!宏来获取一个指向inode_ref.blocks的原始指针
    let header_ptr = core::ptr::addr_of!(inode_ref.block);
    // 使用as操作符来将原始指针转换为*const Ext4ExtentHeader类型
    header_ptr as *mut Ext4ExtentHeader
}

// 假设你已经定义了Ext4ExtentIndex和Ext4ExtentHeader结构体
// 以及to_le16和to_le32函数

// 获取子节点所在的物理块号
#[inline]
pub fn ext4_extent_index_get_leaf(index: &Ext4ExtentIndex) -> u64 {
    ((index.ei_leaf_lo as u64) << 32 | index.ei_leaf_hi as u64) as u64
}

// 设置子节点所在的物理块号
#[inline]
pub fn ext4_extent_index_set_leaf(index: &mut Ext4ExtentIndex, fblock: u64) {
    // 使用transmute函数将u64转换为u16和u32
    unsafe {
        index.ei_leaf_lo = ((fblock << 32) >> 32) as u32;
        index.ei_leaf_hi = (fblock >> 32) as u16;
    }
}

// 获取extent header的魔数
#[inline]
pub fn ext4_extent_header_get_magic(header: &Ext4ExtentHeader) -> u16 {
    header.eh_magic
}

// 设置extent header的魔数
#[inline]
pub fn ext4_extent_header_set_magic(header: &mut Ext4ExtentHeader, magic: u16) {
    header.eh_magic = magic;
}

// 获取extent header的条目数
#[inline]
pub fn ext4_extent_header_get_entries_count(header: &Ext4ExtentHeader) -> u16 {
    header.eh_entries
}

// 设置extent header的条目数
#[inline]
pub fn ext4_extent_header_set_entries_count(header: &mut Ext4ExtentHeader, count: u16) {
    header.eh_entries = count;
}

// 获取extent header的最大条目数
#[inline]
pub fn ext4_extent_header_get_max_entries_count(header: &Ext4ExtentHeader) -> u16 {
    header.eh_max
}

// 设置extent header的最大条目数
#[inline]
pub fn ext4_extent_header_set_max_entries_count(header: &mut Ext4ExtentHeader, max_count: u16) {
    header.eh_max = max_count;
}

// 获取extent header的深度
#[inline]
pub fn ext4_extent_header_get_depth(header: &Ext4ExtentHeader) -> u16 {
    header.eh_depth
}

// 设置extent header的深度
#[inline]
pub fn ext4_extent_header_set_depth(header: &mut Ext4ExtentHeader, depth: u16) {
    header.eh_depth = depth;
}

// 获取extent header的生成号
#[inline]
pub fn ext4_extent_header_get_generation(header: &Ext4ExtentHeader) -> u32 {
    header.eh_generation
}

// 设置extent header的生成号
#[inline]
pub fn ext4_extent_header_set_generation(header: &mut Ext4ExtentHeader, generation: u32) {
    header.eh_generation = generation;
}

pub fn ext4_extent_tree_init(inode_ref: &mut Ext4Inode) {
    /* Initialize extent root header */
    let mut header = unsafe { *ext4_inode_get_extent_header(inode_ref) };

    ext4_extent_header_set_depth(&mut header, 0);
    ext4_extent_header_set_entries_count(&mut header, 0);
    ext4_extent_header_set_generation(&mut header, 0);
    ext4_extent_header_set_magic(&mut header, EXT4_EXTENT_MAGIC);

    let max_entries = EXT4_INODE_BLOCKS * core::mem::size_of::<u32>()
        - core::mem::size_of::<Ext4ExtentHeader>() / core::mem::size_of::<Ext4Extent>();

    ext4_extent_header_set_max_entries_count(&mut header, 4 as u16);

    println!("header {:x?}", header);

    let size = core::mem::size_of::<Ext4ExtentHeader>();
    let count = size / core::mem::size_of::<u32>();
    println!("count {:?}", count);

    copy_header_to_array(&header, &mut inode_ref.block);

    println!("inode_ref.block{:x?}", inode_ref.block);
}

// 定义一个函数，接受一个Ext4ExtentHeader的引用和一个[u32; 15]的可变引用
pub fn copy_header_to_array(header: &Ext4ExtentHeader, array: &mut [u32; 15]) {
    // 使用unsafe代码块，因为涉及到裸指针和类型转换
    unsafe {
        // 把header的引用转换为一个u32的指针
        let header_ptr = header as *const Ext4ExtentHeader as *const u32;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u32; 15] as *mut u32;
        // 使用std::ptr::copy_nonoverlapping函数，从header_ptr拷贝3个u32到array_ptr
        core::ptr::copy_nonoverlapping(header_ptr, array_ptr, 3);
    }
}

pub fn copy_dir_entry_to_array(header: &Ext4DirEntry, array: &mut [u8], offset: usize) {
    // 使用unsafe代码块，因为涉及到裸指针和类型转换
    unsafe {
        // 把header的引用转换为一个u32的指针
        let de_ptr = header as *const Ext4DirEntry as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u8] as *mut u8;

        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), 20);
    }
}

pub fn ext4_inode_init(inode_ref: &mut Ext4Inode, file_type: u16, is_dir: bool) {
    let mut mode = 0 as u16;
    if is_dir {
        mode = 0o777;
        mode |= EXT4_INODE_MODE_DIRECTORY as u16;
    } else if file_type == 0x7 {
        mode = 0o777;
        mode |= EXT4_INODE_MODE_SOFTLINK as u16;
    } else {
        mode = 0o666;
        let t = ext4_fs_correspond_inode_mode(file_type);
        mode |= t.bits();
    }

    inode_ref.ext4_inode_set_flags(EXT4_INODE_FLAG_EXTENTS);
    inode_ref.ext4_inode_set_mode(mode as u16);
    inode_ref.ext4_inode_set_links_cnt(0);
    inode_ref.ext4_inode_set_uid(0);
    inode_ref.ext4_inode_set_gid(0);
    inode_ref.ext4_inode_set_size(8192);
    inode_ref.ext4_inode_set_access_time(0);
    inode_ref.ext4_inode_set_change_inode_time(0);
    inode_ref.ext4_inode_set_modif_time(0);
    inode_ref.ext4_inode_set_del_time(0);
    inode_ref.ext4_inode_set_blocks_count(0);
    inode_ref.ext4_inode_set_flags(0);
    inode_ref.ext4_inode_set_generation(0);
}

pub fn ext4_fs_append_inode_dblk<A: Ext4Traits>(
    inode_ref: &mut Ext4InodeRef,
    iblock: ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
) {
    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;

    ext4_extent_get_blocks_create(inode_ref.inode, iblock, 1, &mut current_fsblk, true);

    current_block = current_fsblk;
    *fblock = current_block;

    println!("current_fsblk {:x?}", current_fsblk);

    let super_block = read_super_block::<A>();
    let inodes_per_group = super_block.inodes_per_group;

    let group = (inode_ref.index - 1) / inodes_per_group;

    ext4_balloc_alloc_block::<A>(inode_ref, group as u64, fblock);
}

pub fn ext4_fs_append_inode_dblk_new<A: Ext4Traits>(
    inode_ref: &mut Ext4InodeRef,
    iblock: ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
) {
    // println!("ext4_fs_append_inode_dblk_new");
    let inode_size = ext4_get_inode_size::<A>();

    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;

    ext4_extent_get_blocks_foo::<A>(inode_ref, iblock, 1, &mut current_fsblk, true, &mut 0);

    current_block = current_fsblk;
    *fblock = current_block;

    println!("fblock {:x?}", fblock);
}

pub fn copy_extent_to_array(extent: &Ext4Extent, array: &mut [u32]) {
    // 使用unsafe代码块，因为涉及到裸指针和类型转换
    unsafe {
        // 把header的引用转换为一个u32的指针
        let extent_ptr = extent as *const Ext4Extent as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u32] as *mut u8;

        let count: usize = core::mem::size_of::<Ext4Extent>() / core::mem::size_of::<u8>();
        core::ptr::copy_nonoverlapping(extent_ptr, array_ptr, count);
    }
}

pub fn ext4_ext_insert_extent(inode_ref: &mut Ext4InodeRef, fblock: &mut ext4_fsblk_t) {
    let mut extent = Ext4Extent::default();

    extent.first_block = 0;
    extent.ee_len = 1;
    extent.ee_start_lo = *fblock as u32;
}

pub fn ext4_extent_get_blocks_create(
    inode: &Ext4Inode,
    iblock: ext4_lblk_t,
    max_blocks: u32,
    result: &mut ext4_fsblk_t,
    extent_create: bool,
) {
    println!("------ext4_extent_get_blocks_create-------");
    let mut vec_extent_path: Vec<Ext4ExtentPath> = Vec::with_capacity(3);

    let mut extent_path = Ext4ExtentPath::default();

    ext4_find_extent_create(inode, iblock, &mut vec_extent_path);

    println!("{:x?}", vec_extent_path);
}

pub fn ext4_find_extent_create(inode: &Ext4Inode, iblock: ext4_lblk_t, v: &mut Vec<Ext4ExtentPath>) {
    let mut eh = &inode.block as *const [u32; 15] as *mut Ext4ExtentHeader;

    let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode.block[..2]);

    let depth = extent_header.eh_depth;
    println!("extent_header {:x?} depth {:x?}", extent_header, depth);

    unsafe {
        (*eh).eh_entries = 1;
    }

    let path_depth = depth + 1;
    let mut search_path = Ext4ExtentPath::default();
    search_path.depth = depth;
    search_path.header = eh;
    search_path.block = Ext4Block::default();
    // search_path.block = iblock;

    let mut i = depth;

    // while i > 1 {
    ext4_ext_binsearch_idx(&mut search_path, iblock);

    println!(" extent {:x?}", unsafe { *(search_path.index) });

    // }
}

pub fn ext4_alloc_new_inode<A: Ext4Traits>() -> u32 {
    let super_block = read_super_block::<A>();

    let ext4_fs = Ext4Fs::<A>::init();

    let gd = ext4_fs_get_block_group_ref::<A>(0);
    let mut gd: GroupDesc = unsafe { *gd };
    // println!("gd {:#x?}", gd);

    let mut free_inodes = ext4_bg_get_free_inodes_count::<A>(&gd);
    let used_dir = ext4_bg_get_used_dirs_count::<A>(&gd);

    // println!(
    //     "free_inodes {:x?}  free_inodes {:?}  used_dir {:x?}",
    //     free_inodes, free_inodes, used_dir
    // );

    let inode_bitmap = ext4_bg_get_inode_bitmap::<A>(&gd);

    // println!("inode bitmap {:x?}", inode_bitmap);

    let inodes_in_bg = ext4_inodes_in_group_cnt::<A>(2);
    // println!("inodes_in_bg {:x?}", inodes_in_bg);

    let bitmap_size = inodes_in_bg / 0x8;
    // println!("bitmap_size = {:x?}", bitmap_size);
    let mut raw_data = A::read_block(inode_bitmap * BLOCK_SIZE);

    let mut data = &mut raw_data[..0x400];

    let mut idx_in_bg = 0 as u32;

    ext4_bmap_bit_find_clr(data, 0, inodes_in_bg, &mut idx_in_bg);

    // println!("inode num {:x?}", idx_in_bg);

    ext4_bmap_bit_set(&mut raw_data, idx_in_bg);

    free_inodes -= 1;

    ext4_bg_set_free_inodes_count::<A>(&mut gd, free_inodes);

    let mut unused = ext4_bg_get_itable_unused::<A>(&gd);

    let free = inodes_in_bg - unused as u32;
    // println!("free {:x?}", free);

    if idx_in_bg >= free {
        unused = inodes_in_bg as u16 - (idx_in_bg + 1) as u16;

        // println!("unused {:x?}", unused);
        ext4_bg_set_itable_unused::<A>(&mut gd, unused);
    }

    let s = read_super_block::<A>();

    let idx = ext4_ialloc_bgidx_to_inode(idx_in_bg, 2, &s);

    idx
}

pub fn ext4_find_new_block<A:Ext4Traits>(goal: ext4_fsblk_t, fblock: &mut ext4_fsblk_t) {
    let super_block = read_super_block::<A>();

    let blocks_per_group = super_block.inodes_per_group;

    let bg_id = goal / blocks_per_group as u64;
    let idx_in_bg = goal % blocks_per_group as u64;

    println!("bg_id {:x?}  idx_in_bg {:x?}", bg_id, idx_in_bg);

    let gd = ext4_fs_get_block_group_ref::<A>(bg_id);
    let mut gd: GroupDesc = unsafe { *gd };

    let block_bitmap = ext4_bg_get_block_bitmap::<A>(&gd);

    let blk_in_bg = ext4_inodes_in_group_cnt::<A>(bg_id as u32);

    let mut raw_data = A::read_block(block_bitmap * BLOCK_SIZE);

    let mut data = &mut raw_data;

    let mut rel_blk_idx = 0 as u32;

    ext4_bmap_bit_find_clr(data, idx_in_bg as u32, blk_in_bg, &mut rel_blk_idx);
}

pub fn ext4_balloc_alloc_block<A: Ext4Traits>(
    inode_ref: &mut Ext4InodeRef,
    goal: ext4_fsblk_t,
    fblock: &mut ext4_fsblk_t,
) {
    let mut alloc: ext4_fsblk_t = 0;
    let mut bmp_blk_adr: ext4_fsblk_t;
    let mut rel_blk_idx: u32 = 0;
    let mut free_blocks: u64;
    let mut r: i32;

    let super_block = read_super_block::<A>();
    let inodes_per_group = super_block.inodes_per_group;
    let blocks_per_group = super_block.inodes_per_group;

    let bg_id = goal / blocks_per_group as u64;
    let idx_in_bg = goal % blocks_per_group as u64;

    let gd = ext4_fs_get_block_group_ref::<A>(bg_id);
    let mut gd: GroupDesc = unsafe { *gd };

    let block_bitmap = ext4_bg_get_block_bitmap::<A>(&gd);
    let blk_in_bg = ext4_inodes_in_group_cnt::<A>(bg_id as u32);
    println!("blk_in_bg {:x?}", blk_in_bg);
    let mut raw_data = A::read_block(block_bitmap * BLOCK_SIZE);

    let r = ext4_balloc_verify_bitmap_csum::<A>(&gd, &raw_data);
    println!("ext4_balloc_verify_bitmap_csum----{:?}", r);
    let mut data: &mut Vec<u8> = &mut raw_data;
    let mut rel_blk_idx = 0 as u32;

    ext4_bmap_bit_find_clr(data, idx_in_bg as u32, 0x8000, &mut rel_blk_idx);

    *fblock = rel_blk_idx as u64;
    ext4_bmap_bit_set(&mut data, rel_blk_idx);

    ext4_balloc_set_bitmap_csum::<A>(&data, &mut gd);
    A::write_block(block_bitmap * BLOCK_SIZE, &data);

    let r = ext4_balloc_verify_bitmap_csum::<A>(&gd, &data);
    println!("ext4_balloc_verify_bitmap_csum----{:?}", r);

    let mut super_blk_free_blocks = super_block.free_blocks_count as u64
        | ((super_block.free_blocks_count_hi as u64) << 32).to_le();
    super_blk_free_blocks -= 1;

    println!(
        "[info] ext4_sb_set_free_blocks_cnt={:x?}",
        super_blk_free_blocks
    );
    ext4_sb_set_free_blocks_cnt::<A>(super_blk_free_blocks as u64);

    let mut inode_blocks = ext4_inode_get_blocks_count(&inode_ref.inode);
    inode_blocks += 8;
    let mut inode_data = read_inode::<A>(inode_ref.index as u64, &super_block);
    inode_data.blocks = inode_blocks as u32;

    ext4_fs_set_inode_checksum::<A>(&mut inode_data, inode_ref.index);

    let block_offset = get_inode_block::<A>(inode_ref.index as u64, &super_block);
    let mut write_back_data = [0u8; 0x80];
    copy_inode_to_array(&inode_data, &mut write_back_data);
    A::write_block(block_offset, &write_back_data);

    let mut fb_cnt = gd.bg_free_blocks_count_lo;
    fb_cnt -= 1;
    gd.bg_free_blocks_count_lo = fb_cnt;

    let csum = ext4_fs_bg_checksum::<A>(bg_id as u32, &mut gd);
    gd.bg_checksum = csum;
    ext4_group_desc_set_bitmap_csum::<A>(bg_id as u32, &mut gd);

    let block_size = BLOCK_SIZE;
    let dsc_cnt = block_size / super_block.desc_size as u64;
    let dsc_id = bg_id / dsc_cnt;
    let first_data_block = super_block.first_data_block;
    let block_id = first_data_block as u64 + dsc_id + 1;
    let offset = (bg_id % dsc_cnt) * super_block.desc_size as u64;
    let mut gd_block_data = A::read_block(block_id as u64 * BLOCK_SIZE);
    let mut gd_data = &mut gd_block_data[offset as usize..offset as usize + size_of::<GroupDesc>()];

    copy_block_group_to_array(&gd, &mut gd_data, 0);
    A::write_block(block_id as u64 * BLOCK_SIZE, &gd_block_data);
    println!("size_of::<GroupDesc>() {:x?}", size_of::<GroupDesc>());

    println!("alloc block num {:x?}", rel_blk_idx);
}

pub fn ext4_inode_get_blocks_count(inode: &Ext4Inode) -> u64 {
    let mut blocks = inode.blocks as u64;
    if inode.osd2.l_i_blocks_high != 0 {
        blocks |= (inode.osd2.l_i_blocks_high as u64) << 32;
    }
    blocks
}
pub fn ext4_sb_set_free_blocks_cnt<A: Ext4Traits>(free_blocks: u64) {
    let mut s = read_super_block::<A>();
    s.free_blocks_count = ((free_blocks << 32) >> 32).to_le() as u32;
    s.free_blocks_count_hi = (free_blocks >> 32) as u32;
    write_super_block::<A>(&s);
}

pub fn write_super_block<A: Ext4Traits>(s: &Ext4SuperBlock) {
    let mut data = A::read_block(BASE_OFFSET);
    copy_super_block_to_array(&s, &mut data);
    // A::write_block(BASE_OFFSET, &data);
}

// 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
pub fn ext4_ext_binsearch_idx(path: &mut Ext4ExtentPath, block: ext4_lblk_t) -> bool {
    // 获取extent header的引用
    let eh = unsafe { &*path.header };

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent_index(eh).add(1) };
    let mut r = unsafe { ext4_last_extent_index(eh) };

    // 如果extent header中没有有效的entry，直接返回false
    if eh.eh_entries == 0 {
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
    path.index = unsafe { l.sub(1) };
    // 返回true，表示找到了对应的extent
    true
}

// 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
pub fn ext4_ext_binsearch_idx_foo(path: &mut ext4_extent_path, block: ext4_lblk_t) -> bool {
    // 获取extent header的引用
    let eh = path.header;

    // 定义左右两个指针，分别指向第一个和最后一个extent
    let mut l = unsafe { ext4_first_extent_index_mut(eh).add(1) };
    let mut r = unsafe { ext4_last_extent_index_mut(eh) };

    // 如果extent header中没有有效的entry，直接返回false
    unsafe {
        if (*eh).eh_entries == 0 {
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

// 假设我们已经导入了一些必要的模块，如std::mem, std::ptr, std::slice等

// ext4_block_group结构
#[repr(C)]
struct ext4_block_group {
    bg_block_bitmap_lo: u32,      // 块位图的低32位物理块号
    bg_inode_bitmap_lo: u32,      // inode位图的低32位物理块号
    bg_inode_table_lo: u32,       // inode表的低32位物理块号
    bg_free_blocks_count_lo: u16, // 空闲块数的低16位
    bg_free_inodes_count_lo: u16, // 空闲inode数的低16位
    bg_used_dirs_count_lo: u16,   // 使用的目录数的低16位
    bg_flags: u16,                // 块组标志
    bg_exclude_bitmap_lo: u32,    // 排除位图的低32位物理块号
    bg_block_bitmap_csum_lo: u16, // 块位图校验和的低16位
    bg_inode_bitmap_csum_lo: u16, // inode位图校验和的低16位
    bg_itable_unused_lo: u16,     // 未使用的inode表项数的低16位
    bg_checksum: u16,             // 块组描述符校验和
    bg_block_bitmap_hi: u32,      // 块位图的高32位物理块号
    bg_inode_bitmap_hi: u32,      // inode位图的高32位物理块号
    bg_inode_table_hi: u32,       // inode表的高32位物理块号
    bg_free_blocks_count_hi: u16, // 空闲块数的高16位
    bg_free_inodes_count_hi: u16, // 空闲inode数的高16位
    bg_used_dirs_count_hi: u16,   // 使用的目录数的高16位
    bg_itable_unused_hi: u16,     // 未使用的inode表项数的高16位
    bg_exclude_bitmap_hi: u32,    // 排除位图的高32位物理块号
    bg_block_bitmap_csum_hi: u16, // 块位图校验和的高16位
    bg_inode_bitmap_csum_hi: u16, // inode位图校验和的高16位
    bg_reserved: [u32; 3],        // 保留字段
}

// 假设我们已经导入了一些必要的模块，如std::mem, std::ptr, std::slice等

// ext4_inode_ref结构
struct ext4_inode_ref {
    block_group_ref: *mut ext4_block_group_ref, // 块组描述符的引用
    inode: *mut Ext4Inode,                      // inode的指针
    index: u32,                                 // inode在块组中的索引
    dirty: bool,                                // 是否需要写回
}

// ext4_extent_path结构
#[derive(Debug, Clone, Copy)]
struct ext4_extent_path {
    header: *mut Ext4ExtentHeader, // extent头部的指针
    extent: *mut Ext4Extent,       // extent的指针
    index: *mut Ext4ExtentIndex,   // extent索引的指针
    block: ext4_fsblk_t,           // extent所在的物理块号
    depth: u16,                    // extent在树中的深度
    maxdepth: u16,
}

impl ext4_extent_path {
    pub fn new() -> Self {
        Self {
            header: core::ptr::null_mut(),
            extent: core::ptr::null_mut(),
            index: core::ptr::null_mut(),
            block: 0,
            depth: 0,
            maxdepth: 0,
        }
    }
}

// block_group_ref结构
struct ext4_block_group_ref {
    block_group: *mut ext4_block_group, // 块组描述符的指针
    block: ext4_fsblk_t,                // 块组描述符所在的物理块号
    // buf: *mut ext4_buf, // 块组描述符所在的缓冲区
    dirty: bool, // 是否需要写回
}

pub const EOK: i32 = 0;

pub fn ext_depth(inode: &Ext4Inode) -> u16 {
    let header = ext_inode_hdr(inode);
    unsafe { (*header).eh_depth }
}

// ext4_find_extent函数
pub fn ext4_find_extent_foo(
    inode_ref: &mut Ext4InodeRef,
    block: ext4_lblk_t,
    path: &mut Vec<ext4_extent_path>,
    flags: i32,
) -> i32 {
    // println!("ext4_find_extent_foo");
    // 初始化一些变量
    let mut err: i32 = EOK;
    let mut depth: u16 = 0;
    let mut p: *mut ext4_extent_path;
    let mut eh: *mut Ext4ExtentHeader;
    let mut ex: *mut Ext4Extent;
    let mut ei: *mut Ext4ExtentIndex;
    let mut block_buf: *mut ext4_buf;
    let mut block_nr: ext4_fsblk_t;
    let mut i: i32;

    // let mut path = orig_path;

    let mut block_buf = ext4_buf::new(block as u64);

    println!("inode block{:x?}", inode_ref.inode.block);

    let mut eh = &inode_ref.inode.block as *const [u32; 15] as *mut Ext4ExtentHeader;

    depth = ext_depth(inode_ref.inode);

    // 如果没有传入路径，分配一个新的路径
    if path.is_empty() {
        let path_depth = depth + 1;
        // 使用vec来分配一块内存，并将其初始化为零
        let mut vec = vec![ext4_extent_path::new(); (path_depth + 1) as usize];

        // 设置路径
        path.append(&mut vec);

        // println!("-----path is null--- path len {:x?}", path);

        path[0].maxdepth = path_depth;
    }

    path[0].header = eh;
    path[0].depth = depth;

    let mut ppos = 0 as usize;

    let mut i = depth;

    while i > 0 {
        ext4_ext_binsearch_idx_foo(&mut path[ppos], block);
        path[ppos].block = ext4_idx_pblock(path[ppos].index);
        path[ppos].depth = i as u16;
        path[ppos].extent = core::ptr::null_mut();

        // 获取索引指向的子节点的物理块号
        block_nr = path[ppos].block;

        i -= 1;
        ppos += 1;
    }

    path[ppos].depth = i;
    path[ppos].extent = core::ptr::null_mut();
    path[ppos].index = core::ptr::null_mut();

    ext4_ext_binsearch_foo(&mut path[ppos], block);

    // 获取最后一个节点的extent
    ex = ext4_ext_find_extent(inode_ref, eh, block);

    if ex.is_null() {
        println!("ext4_ext_find_extent ex is null");
    } else {
        println!("ext4_ext_find_extent ex not null ex {:x?}", ex);
    }

    // 设置最后一个元素的extent
    path[depth as usize].extent = ex;
    println!("path {:x?}", path[depth as usize]);

    if path[ppos].extent != core::ptr::null_mut() {
        let block = ext4_ext_pblock(path[ppos].extent);
        path[ppos].block = block as _;
    }

    return EOK;
}

pub fn ext4_ext_pblock(ex: *mut Ext4Extent) -> u32 {
    let mut block = 0;

    unsafe {
        block = (*ex).ee_start_lo;
        block |= (((*ex).ee_start_hi as u32) << 31) << 1;
    }

    block
}

pub fn ext4_ext_pblock_foo(ex: &Ext4Extent) -> u32 {
    let mut block = 0;

    unsafe {
        block = ex.ee_start_lo;
        block |= ((ex.ee_start_hi as u32) << 31) << 1;
    }

    block
}

// ext4_idx_pblock函数
pub fn ext4_idx_pblock(idx: *mut Ext4ExtentIndex) -> ext4_fsblk_t {
    // 如果索引为空，返回0
    if idx.is_null() {
        return 0;
    }

    // 获取索引的低32位物理块号
    let mut pblock = unsafe { (*idx).ei_leaf_lo } as u64;

    // 如果支持64位物理块号，获取索引的高16位物理块号
    // if ext4_has_feature_64bit(sb) {
    let pblock_hi = unsafe { (*idx).ei_leaf_hi };
    pblock |= ((pblock_hi as ext4_fsblk_t) << 32) as u64;
    // }

    // 返回索引的物理块号
    return pblock;
}

// ext_block_hdr函数
pub fn ext_block_hdr(buf: *mut ext4_buf) -> *mut Ext4ExtentHeader {
    // 如果数据块为空，返回空指针
    if buf.is_null() {
        return ptr::null_mut();
    }

    // 获取数据块的数据指针
    let mut data: &mut [u8] = unsafe { &mut (*buf).data };
    // let v = data.as_mut_slice();
    let mut eh = data as *mut [u8] as *mut Ext4ExtentHeader;
    eh
}

// ext4_ext_find_extent函数
pub fn ext4_ext_find_extent(
    inode_ref: &mut Ext4InodeRef,
    eh: *mut Ext4ExtentHeader,
    block: ext4_lblk_t,
) -> *mut Ext4Extent {
    // 初始化一些变量
    let mut low: i32;
    let mut high: i32;
    let mut mid: i32;
    let mut ex: *mut Ext4Extent;

    // 如果头部的extent数为0，返回空指针
    if eh.is_null() || unsafe { (*eh).eh_entries } == 0 {
        return ptr::null_mut();
    }

    // 从头部获取第一个extent的指针
    ex = ext4_first_extent_mut(eh);

    // 如果头部的深度不为0，返回空指针
    if unsafe { (*eh).eh_depth } != 0 {
        return ptr::null_mut();
    }

    // 使用二分查找法在extent数组中查找逻辑块号
    low = 0;
    high = unsafe { (*eh).eh_entries - 1 } as i32;
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

// ext4_ext_find_index函数
pub fn ext4_ext_find_index(
    inode_ref: &mut Ext4InodeRef,
    eh: *mut Ext4ExtentHeader,
    block: ext4_lblk_t,
) -> *mut Ext4ExtentIndex {
    // 初始化一些变量
    let mut low: i32;
    let mut high: i32;
    let mut mid: i32;
    let mut ei: *mut Ext4ExtentIndex;

    // 如果头部的索引数为0，返回空指针
    if eh.is_null() || unsafe { (*eh).eh_entries } == 0 {
        return ptr::null_mut();
    }

    // 从头部获取第一个索引的指针
    ei = ext4_first_extent_index_mut(eh);

    // 如果头部的深度为0，返回空指针
    if unsafe { (*eh).eh_depth } == 0 {
        return ptr::null_mut();
    }

    // 使用二分查找法在索引数组中查找逻辑块号
    low = 0;
    high = unsafe { (*eh).eh_entries - 1 } as i32;
    while low <= high {
        // 计算中间位置
        mid = (low + high) / 2;

        // 获取中间位置的索引的指针
        ei = unsafe { ei.add(mid as usize) };

        // 比较索引的逻辑块号和目标逻辑块号
        if block > unsafe { (*ei).ei_leaf_lo } {
            // 如果目标逻辑块号大于索引的逻辑块号，说明目标在右半部分
            low = mid + 1;
        } else if block < unsafe { (*ei).ei_leaf_lo } {
            // 如果目标逻辑块号小于索引的逻辑块号，说明目标在左半部分
            high = mid - 1;
        } else {
            // 如果目标逻辑块号等于索引的逻辑块号，说明找到了目标，返回索引的指针
            return ei;
        }
    }

    // 如果没有找到目标，返回最后一个小于目标的索引的指针
    if high < 0 {
        return ptr::null_mut();
    } else {
        return unsafe { ei.add(high as usize) };
    }
}

// 获取一个逻辑块号对应的物理块号，如果需要的话，还可以分配新的物理块
pub fn ext4_extent_get_blocks_foo<A:Ext4Traits>(
    inode_ref: &mut Ext4InodeRef,
    iblock: ext4_lblk_t,
    max_blocks: u32,
    result: &mut ext4_fsblk_t,
    create: bool,
    blocks_count: &mut u32,
) {
    let mut path: Vec<ext4_extent_path> = Vec::new();
    let mut newex: Ext4Extent = Ext4Extent::default();

    let mut ex: Option<&Ext4Extent>;
    let mut goal: ext4_fsblk_t;
    let mut err: i32 = EOK;
    let mut depth: u16;
    let mut allocated: u32 = 0;
    let mut next: ext4_lblk_t;
    let mut newblock: ext4_fsblk_t;

    // 设置结果为0
    *result = 0;
    *blocks_count = 0;

    // 查找这个逻辑块号所在的extent
    err = ext4_find_extent_foo(inode_ref, iblock, &mut path, 0);

    depth = ext_depth(inode_ref.inode);

    let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode_ref.inode.block[..2]);
    println!("header {:x?}", extent_header);

    let ex = path[depth as usize].extent;

    if ex != core::ptr::null_mut() {
        println!("ex is not null iblock={:x?}", iblock);
        let ee_block = unsafe { (*ex).first_block };
        let ee_start = unsafe { (*ex).ee_start_lo | ((((*ex).ee_start_hi as u32) << 31) << 1) };
        let ee_len = unsafe { (*ex).ee_len };

        if iblock >= ee_block && iblock <= (ee_block as i32 + ee_len as i32 - 1) as u32 {
            allocated = ee_len as u32 - (iblock - ee_block);

            newblock = iblock as u64 - ee_block as u64 + ee_start as u64;

            if allocated > max_blocks {
                allocated = max_blocks;
            }

            *result = newblock;
            println!("go out");
            return;
        } else {
            // newblock = iblock - ee_block + ee_start;
        }
    }

    let next = ext4_ext_next_allocated_block(&path[0]);

    // println!("next={:x?}", next);

    allocated = next - iblock;

    if allocated > max_blocks {
        allocated = max_blocks;
    }

    // println!("/* allocate new block */");

    // let goal = ext4_ext_find_goal(inode_ref, &mut path[0], iblock);

    let goal = 0;

    let mut alloc_block = 0;
    ext4_balloc_alloc_block::<A>(inode_ref, goal as u64, &mut alloc_block);

    println!("alloc_block {:x?}", alloc_block);

    newex.first_block = iblock;
    newex.ee_start_lo = alloc_block as u32 & 0xffffffff;
    newex.ee_start_hi = (((alloc_block as u32) << 31) << 1) as u16;
    newex.ee_len = allocated as u16;

    // if path[0].extent.is_null() {
    // }

    ext4_ext_insert_extent_foo::<A>(inode_ref, &mut path[0], &newex, 0);

    newblock = ext4_ext_pblock(path[0].extent) as u64;

    *result = newblock;
}

const EXT_UNWRITTEN_MAX_LEN: u16 = 65535;

pub fn ext4_ext_get_actual_len(ext: &Ext4Extent) -> u16 {
    // 返回extent的实际长度
    if ext.ee_len <= EXT_INIT_MAX_LEN {
        ext.ee_len
    } else {
        ext.ee_len - EXT_INIT_MAX_LEN
    }
}

// 定义ext4_ext_can_prepend函数
pub fn ext4_ext_can_prepend(ex1: &Ext4Extent, ex2: &Ext4Extent) -> bool {
    // 检查是否可以将ex2合并到ex1的前面
    if ext4_ext_pblock_foo(ex2) + ext4_ext_get_actual_len(ex2) as u32 != ext4_ext_pblock_foo(ex1) {
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

pub fn ext4_ext_insert_extent_foo<A: Ext4Traits>(
    inode_ref: &mut Ext4InodeRef,
    path: &mut ext4_extent_path,
    newext: &Ext4Extent,
    flags: i32,
) {
    let mut depth = ext_depth(inode_ref.inode);
    let mut level = 0;
    let mut ret = 0;
    let mut npath: Option<ext4_extent_path> = None;
    let mut ins_right_leaf = false;
    let mut need_split = false;

    ext4_ext_insert_leaf(inode_ref, path, depth, newext, flags, &mut need_split);

    let inode_data = inode_ref.inode;
    let super_block = read_super_block::<A>();
    let block_offset = get_inode_block::<A>(inode_ref.index as u64, &super_block);
    let mut write_back_data = [0u8; 0x80];
    copy_inode_to_array(&inode_data, &mut write_back_data);
    A::write_block(block_offset, &write_back_data);

    let eh = path.header;

    let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode_data.block);
}

const EXT_INIT_MAX_LEN: u16 = 32768;

pub fn ext4_ext_can_append(ex1: &Ext4Extent, ex2: &Ext4Extent) -> bool {
    // println!("ext4_ext_can_append?");
    // println!(
    //     "pblock1 {:x?} pblock2 {:x?}",
    //     ext4_ext_pblock_foo(ex1),
    //     ext4_ext_pblock_foo(ex2)
    // );
    // println!(
    //     "len1 {:x?} len2 {:x?}",
    //     ext4_ext_get_actual_len(ex1),
    //     ext4_ext_get_actual_len(ex2)
    // );
    // println!(
    //     "first_block1 {:x?} first_block2 {:x?}",
    //     ex1.first_block, ex2.first_block
    // );

    if ext4_ext_pblock_foo(ex1) + ext4_ext_get_actual_len(ex1) as u32 != ext4_ext_pblock_foo(ex2) {
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

pub const EIO: i32 = 5;

pub fn ext4_ext_insert_leaf(
    inode_ref: &mut Ext4InodeRef,
    path: &mut ext4_extent_path,
    depth: u16,
    newext: &Ext4Extent,
    flags: i32,
    need_split: &mut bool,
) -> i32 {
    let eh = path.header;
    let ex = path.extent;
    let last_ex = ext_last_extent(eh);

    let mut diskblock = 0;
    diskblock = newext.ee_start_lo;
    diskblock |= ((newext.ee_start_hi as u32) << 31) << 1;

    println!("insert newext {:x?}", newext);

    unsafe {
        if !ex.is_null() && ext4_ext_can_append(&*(path.extent), newext) {
            println!("can append");
            if ext4_ext_is_unwritten(&*(path.extent)) {
                ext4_ext_mark_unwritten((*path).extent);
            }
            (*(path.extent)).ee_len =
                ext4_ext_get_actual_len(&*(path.extent)) + ext4_ext_get_actual_len(&newext);
            // (*(path.extent)).first_block = newext.first_block;
            // (*(path.extent)).ee_start_lo = newext.ee_start_lo;
            // (*(path.extent)).ee_start_hi = newext.ee_start_hi;

            (*path).block = diskblock as u64;
            return EOK;
        }

        if !ex.is_null() && ext4_ext_can_prepend(&*(path.extent), newext) {
            println!("can preappend");
            // (*(path.extent)).first_block = newext.first_block;
            (*(path.extent)).ee_len =
                ext4_ext_get_actual_len(&*(path.extent)) + ext4_ext_get_actual_len(&newext);
            // (*(path.extent)).ee_start_lo = newext.ee_start_lo;
            // (*(path.extent)).ee_start_hi = newext.ee_start_hi;
            (*path).block = diskblock as u64;

            if ext4_ext_is_unwritten(&*(path.extent)) {
                ext4_ext_mark_unwritten((*path).extent);
            }
            return EOK;
        }
    }

    if ex.is_null() {
        println!("-------ex is null set first_index");

        let first_extent = ext_first_extent_foo(eh);

        unsafe {
            (*path).extent = first_extent;
            println!("first_extent {:x?}", first_extent);
        }
        unsafe {
            if (*eh).eh_entries == (*eh).eh_max {
                *need_split = true;
                println!("need split");
                return EIO;
            } else {
                (*(path.extent)).ee_len = newext.ee_len;
                (*(path.extent)).ee_start_lo = newext.ee_start_lo;
                (*(path.extent)).ee_start_hi = newext.ee_start_hi;
                (*(path.extent)).first_block = newext.first_block;
            }
        }
    }

    unsafe {
        if (*eh).eh_entries == (*eh).eh_max {
            *need_split = true;

            (*(path.extent)).ee_len = newext.ee_len;
            (*(path.extent)).first_block = newext.first_block;
            (*(path.extent)).ee_start_lo = newext.ee_start_lo;
            (*(path.extent)).ee_start_hi = newext.ee_start_hi;

            (*path).block = diskblock as u64;
            println!("need split");
            return EIO;
        } else {
            if ex.is_null() {
                let first_extent = ext_first_extent_foo(eh);
                (*path).extent = first_extent;

                (*(path.extent)).ee_len = newext.ee_len;
                (*(path.extent)).first_block = newext.first_block;
                (*(path.extent)).ee_start_lo = newext.ee_start_lo;
                (*(path.extent)).ee_start_hi = newext.ee_start_hi;
            } else if newext.first_block > (*(path.extent)).first_block {
                // insert after
                let next_extent = ex.add(1);
                (*path).extent = next_extent;
            } else {
            }
        }
    }

    // let len = ext_last_extent(eh) as usize - (ex as usize) + 1;

    // assert!(len >= 0 );

    unsafe {
        (*(path.extent)).first_block = newext.first_block;
        (*(path.extent)).ee_len = newext.ee_len;

        (*(path.extent)).ee_start_lo = newext.ee_start_lo;
        (*(path.extent)).ee_start_hi = newext.ee_start_hi;
    }

    println!("--------------entries_count+=1\n");
    unsafe {
        (*eh).eh_entries += 1;
    }

    println!(
        "after_path.extent{:x?} header {:x?}",
        unsafe { *(path).extent },
        unsafe { *(path).header }
    );

    return EOK;
}

pub fn ext4_ext_is_unwritten(ext: &Ext4Extent) -> bool {
    // 返回extent是否是未写入的
    ext.ee_len > EXT_INIT_MAX_LEN
}

pub fn ext4_ext_mark_unwritten(ext: *mut Ext4Extent) {
    // 返回extent是否是未写入的
    unsafe {
        (*ext).ee_len |= EXT_INIT_MAX_LEN;
    }
}

// ext4_ext_find_goal函数
pub fn ext4_ext_find_goal<A:Ext4Traits>(
    inode_ref: &mut Ext4InodeRef,
    path: &mut ext4_extent_path,
    block: ext4_lblk_t,
) -> ext4_fsblk_t {
    // 获取路径的深度
    let depth = path.depth;
    // 获取路径的extent
    let ex = path.extent;

    // 如果extent不为空
    if !ex.is_null() {
        // 获取extent的物理块号
        let ext_pblk = ext4_ext_pblock(ex);
        // 获取extent的逻辑块号
        let ext_block = unsafe { (*ex).first_block };

        // 如果目标逻辑块号大于extent的逻辑块号，返回extent的物理块号加上差值
        if block > ext_block {
            return (ext_pblk + (block - ext_block)) as u64;

        // 如果目标逻辑块号小于extent的逻辑块号，返回extent的物理块号减去差值
        } else {
            return (ext_pblk - (ext_block - block)) as u64;
        }
    }

    let super_block = read_super_block::<A>();

    let grp_inodes = super_block.inodes_per_group;

    return ((inode_ref.index - 1) / grp_inodes) as u64;
}

pub fn ext4_ext_next_allocated_block_foo(path: &Ext4ExtentPath) -> u32 {
    let mut depth = path.depth;

    if depth == 0 && path.extent == core::ptr::null_mut() {
        return u32::MAX - 1;
    }

    // 从最深的节点开始，向上查找
    while depth >= 0 {
        // 如果是叶子节点
        if depth == path.depth {
            // 获取当前节点的extent
            let extent = path.extent;

            // 如果extent不为空且不是最后一个extent，返回下一个extent的第一个逻辑块号
            if !extent.is_null() && extent != ext_last_extent(path.header) {
                println!("-------leaf---------");
                return unsafe { (*extent.add(1)).first_block };
            }
        } else {
            // 如果是索引节点
            // 获取当前节点的索引
            let index = path.index;

            // 如果索引不为空且不是最后一个索引，返回下一个索引的第一个逻辑块号
            if !index.is_null() && index != ext_last_index(path.header) {
                println!("-------index---------");
                return unsafe { (*index.add(1)).first_block };
            }
        }
    }

    u32::MAX - 1
}

pub fn ext_last_extent(eh: *const Ext4ExtentHeader) -> *mut Ext4Extent {
    // 如果头部为空，返回空指针
    if eh.is_null() {
        return ptr::null_mut();
    }

    // 获取头部的extent数
    let count = unsafe { (*eh).eh_entries };

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

    // 获取头部的extent数
    let count = unsafe { (*eh).eh_entries };

    // 如果extent数为0，返回空指针
    if count == 0 {
        return ptr::null_mut();
    }

    // 返回头部中第一个extent的指针，即头部的指针加上头部的大小
    return unsafe { (eh as *mut u8).add(mem::size_of::<Ext4ExtentHeader>()) as *mut Ext4Extent };
}

// ext_first_extent函数
pub fn ext_first_extent_foo(eh: *const Ext4ExtentHeader) -> *mut Ext4Extent {
    // 如果头部为空，返回空指针
    if eh.is_null() {
        return ptr::null_mut();
    }

    // 返回头部中第一个extent的指针，即头部的指针加上头部的大小
    return unsafe { (eh as *mut u8).add(mem::size_of::<Ext4ExtentHeader>()) as *mut Ext4Extent };
}

pub fn ext_last_index(eh: *const Ext4ExtentHeader) -> *mut Ext4ExtentIndex {
    // 如果头部为空，返回空指针
    if eh.is_null() {
        return ptr::null_mut();
    }

    // 获取头部的索引数
    let count = unsafe { (*eh).eh_entries };

    // 如果索引数为0，返回空指针
    if count == 0 {
        return ptr::null_mut();
    }

    // 获取头部中第一个索引的指针
    let first = ext_first_index(eh);

    // 返回头部中最后一个索引的指针，即第一个索引的指针加上索引数减一
    return unsafe { first.add((count - 1) as usize) };
}

pub fn ext_first_index(eh: *const Ext4ExtentHeader) -> *mut Ext4ExtentIndex {
    // 如果头部为空，返回空指针
    if eh.is_null() {
        return ptr::null_mut();
    }

    // 获取头部的extent数
    let count = unsafe { (*eh).eh_entries };

    // 如果extent数为0，返回空指针
    if count == 0 {
        return ptr::null_mut();
    }

    // 返回头部中第一个extent的指针，即头部的指针加上头部的大小
    return unsafe {
        (eh as *mut u8).add(mem::size_of::<Ext4ExtentHeader>()) as *mut Ext4ExtentIndex
    };
}

pub fn ext4_fs_get_inode_dblk_idx_internal(
    inode_ref: &Ext4Inode,
    iblock: ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
    extent_create: bool,
) {
}

pub fn ext4_fs_init_inode_dblk_idx(
    inode_ref: &Ext4Inode,
    iblock: ext4_lblk_t,
    fblock: &mut ext4_fsblk_t,
    extent_create: bool,
    support_unwritten: bool,
) {
    let mut current_block: ext4_fsblk_t;
    let mut current_fsblk: ext4_fsblk_t = 0;
    ext4_extent_get_blocks(inode_ref, iblock, 1, &mut current_fsblk, false);

    current_block = current_fsblk;
    *fblock = current_block;
}

pub fn ext4_file_write<A: Ext4Traits>(ext4_file: &mut Ext4File, data: &[u8], size: u64) {
    let s = read_super_block::<A>();
    let block_size = ext4_sb_get_block_size::<A>() as u64;
    let iblock_last = ext4_file.fpos + size / block_size;
    let mut iblk_idx = ext4_file.fpos / block_size;
    let ifile_blocks = ext4_file.fsize + block_size - 1 / block_size;

    println!("blocks_last {:?}", iblock_last);

    let mut fblock: ext4_fsblk_t = 0;

    let mut inode_data = read_inode::<A>(ext4_file.inode as u64, &s);

    println!("inode data {:x?}", inode_data);

    let mut inode_ref = Ext4InodeRef {
        inode: &inode_data,
        index: ext4_file.inode,
        dirty: false,
    };

    while iblk_idx < iblock_last {
        ext4_fs_append_inode_dblk::<A>(&mut inode_ref, iblk_idx as u32, &mut fblock);

        iblk_idx += 1;
    }

    // insert extent
    let mut extent = Ext4Extent::default();
    extent.first_block = 0;
    extent.ee_len = 1;
    extent.ee_start_lo = fblock as u32;

    let mut data = &mut inode_data.block[3..];
    // let mut data = &inode.block[3..];
    copy_extent_to_array(&extent, data);

    println!("----------------inode block {:x?}", inode_data.block);
    let block_offset = get_inode_block::<A>(ext4_file.inode as u64, &s);
    let mut bytes = [0u8; 0x80];
    copy_inode_to_array(&inode_data, &mut bytes);

    A::write_block(block_offset, &bytes);
}

pub fn ext4_bg_get_block_bitmap<A: Ext4Traits>(bg: &GroupDesc) -> u64 {
    let s = read_super_block::<A>();

    let mut v = u32::from_le(bg.bg_block_bitmap_lo) as u64;

    let desc_size = ext4_sb_get_desc_size::<A>();
    if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
        v |= (u32::from_le(bg.bg_block_bitmap_hi) as u64) << 32;
    }

    v
}

pub fn copy_inode_to_array(inode: &Ext4Inode, array: &mut [u8]) {
    // 使用unsafe代码块，因为涉及到裸指针和类型转换
    unsafe {
        // 把header的引用转换为一个u32的指针
        let inode_ptr = inode as *const Ext4Inode as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u8] as *mut u8;
        core::ptr::copy_nonoverlapping(inode_ptr, array_ptr, 0x9c);
    }
}

pub fn copy_diren_to_array(inode: &Ext4DirEntry, array: &mut [u8]) {
    unsafe {
        // 把header的引用转换为一个u32的指针
        let inode_ptr = inode as *const Ext4DirEntry as *const u8;
        // 把array的可变引用转换为一个u32的可变指针
        let array_ptr = array as *mut [u8] as *mut u8;
        core::ptr::copy_nonoverlapping(inode_ptr, array_ptr, 0x9c);
    }
}

pub fn ext4_file_create<A: Ext4Traits>(path: &str) {
    let name_len = path.len() as u32;
    let mp = Ext4MountPoint::new("/");

    let super_block = read_super_block::<A>();

    //alloc new inode
    let idx = ext4_alloc_new_inode::<A>();

    //config inode data
    let mut inode_data = Ext4Inode::default();
    ext4_inode_init(&mut inode_data, 0x1, false);
    inode_data.ext4_inode_set_flags(0x80000);
    ext4_extent_tree_init(&mut inode_data);

    //write back to device
    let block_offset = get_inode_block::<A>(idx as u64, &super_block);
    let mut write_back_data = [0u8; 0x80];
    copy_inode_to_array(&inode_data, &mut write_back_data);
    A::write_block(block_offset, &write_back_data);

    //link to mp
    let mut new_inode_data = read_inode::<A>(idx as u64, &super_block);
    let mut child_inode_ref = Ext4InodeRef {
        inode: &mut new_inode_data,
        index: idx,
        dirty: false,
    };
    let root_inode = read_inode::<A>(2, &super_block);
    ext4_link::<A>(
        &mp,
        &root_inode,
        &mut child_inode_ref,
        path,
        name_len,
        false,
    );

    // set extent
    let mut ext4_file = Ext4File::new(mp);
    ext4_file.inode = idx;
    let size = 4096;
    let block_size = ext4_sb_get_block_size::<A>() as u64;
    let iblk_idx = ext4_file.fpos / block_size;

    let mut fblock: ext4_fsblk_t = 0;
    ext4_fs_append_inode_dblk::<A>(&mut child_inode_ref, iblk_idx as u32, &mut fblock);

    let mut extent = Ext4Extent::default();
    extent.first_block = 0;
    extent.ee_len = 1;
    extent.ee_start_lo = fblock as u32;
    let data = &mut inode_data.block[3..];
    copy_extent_to_array(&extent, data);
    let block_offset = get_inode_block::<A>(ext4_file.inode as u64, &super_block);
    let mut bytes = [0u8; 0x80];
    copy_inode_to_array(&inode_data, &mut bytes);
    A::write_block(block_offset, &bytes);
}

pub fn ext4_file_write_new<A: Ext4Traits>(path: &str, data: &[u8], size: u64) {
    let s = read_super_block::<A>();
    let mp = Ext4MountPoint::new("/");
    let mut ext4_file = Ext4File::new(mp);
    ext4_generic_open::<A>(&mut ext4_file, path);

    let inode = ext4_file.inode;

    let inode_data = read_inode::<A>(inode as u64, &s);

    let mut extents: Vec<Ext4Extent> = Vec::new();
    ext4_find_extent::<A>(&inode_data, &mut extents);

    let fblock = extents[0].ee_start_lo as u64;

    A::write_block(fblock * BLOCK_SIZE, &data[..BLOCK_SIZE as usize]);
}

pub fn ext4_get_inode_size<A: Ext4Traits>() -> u16 {
    let super_block = read_super_block::<A>();
    super_block.inode_size
}

pub fn ext4_inode_alloc<A: Ext4Traits>(file_type: u16) -> u32 {
    let super_block = read_super_block::<A>();
    let is_dir = file_type == 2;
    let inode_size = ext4_get_inode_size::<A>();
    let mut index = 0;
    let group = 2;

    let mut bgid = 0;
    let bg_count = ext4_block_group_cnt::<A>();

    let s_free_inodes: u32 = super_block.free_inodes_count;

    let mut idx = 0;

    while bgid <= bg_count {
        if bgid == bg_count {
            bgid = 0;

            continue;
        }

        let gd = ext4_fs_get_block_group_ref::<A>(bgid as u64);
        let mut gd: GroupDesc = unsafe { *gd };

        let mut free_inodes = ext4_bg_get_free_inodes_count::<A>(&gd);

        if free_inodes > 0 {
            let inode_bitmap = ext4_bg_get_inode_bitmap::<A>(&gd);

            let mut raw_data = A::read_block(inode_bitmap * BLOCK_SIZE);

            let inodes_in_bg = ext4_inodes_in_group_cnt::<A>(bgid);

            let bitmap_size: u32 = inodes_in_bg / 0x8;

            let data = &mut raw_data[..bitmap_size as usize];

            let mut idx_in_bg = 0 as u32;
            ext4_bmap_bit_find_clr(data, 0, inodes_in_bg, &mut idx_in_bg);

            ext4_bmap_bit_set(&mut raw_data, idx_in_bg);

            ext4_ialloc_set_bitmap_csum::<A>(&raw_data, &mut gd);

            ext4_group_desc_set_bitmap_csum::<A>(bgid, &mut gd);
            ext4_block_group_des_write_back::<A>(bgid as u64, gd);
            let gd = ext4_fs_get_block_group_ref::<A>(bgid as u64);
            let mut gd: GroupDesc = unsafe { *gd };
            let r = ext4_fs_verify_bg_csum::<A>(bgid, &mut gd);

            /* Modify filesystem counters */
            free_inodes -= 1;
            ext4_bg_set_free_inodes_count::<A>(&mut gd, free_inodes);

            /* Decrease unused inodes count */
            let mut unused = ext4_bg_get_itable_unused::<A>(&gd) as u32;
            let free = inodes_in_bg - unused as u32;
            if idx_in_bg >= free {
                unused = inodes_in_bg - (idx_in_bg + 1);
                ext4_bg_set_itable_unused::<A>(&mut gd, unused as u16);
                println!("unused={:x?}", gd.bg_itable_unused_lo);
            }

            /* Save modified block group */
            ext4_group_desc_set_bitmap_csum::<A>(bgid, &mut gd);
            ext4_block_group_des_write_back::<A>(bgid as u64, gd);
            let gd = ext4_fs_get_block_group_ref::<A>(bgid as u64);
            let mut gd: GroupDesc = unsafe { *gd };
            let r = ext4_fs_verify_bg_csum::<A>(bgid, &mut gd);
            println!("4-------------ext4_fs_verify_bg_csum {:?}", r);

            /* Update superblock */
            let mut s: Ext4SuperBlock = read_super_block::<A>();
            s.free_inodes_count -= 1;
            let mut data = A::read_block(BASE_OFFSET);
            copy_super_block_to_array(&s, &mut data);
            // A::write_block(BASE_OFFSET, &data);

            let mut s: Ext4SuperBlock = read_super_block::<A>();
            println!("sb free inodes {:x?}", s.free_inodes_count);

            /* Compute the absolute i-nodex number */
            let idx = ext4_ialloc_bgidx_to_inode(idx_in_bg, bgid, &super_block);
            println!("alloc inode idx {:x?}", idx);

            return idx;
        }
    }

    idx
}

pub fn ext4_group_desc_set_bitmap_csum<A:Ext4Traits>(bgid: u32, gd: &mut GroupDesc) {
    let csum = ext4_fs_bg_checksum::<A>(bgid, gd);

    (*gd).bg_checksum = csum;

    println!("bg csum {:x?}  csum {:x?}", gd.bg_checksum, csum);
}

pub fn ext4_balloc_set_bitmap_csum<A: Ext4Traits>(bitmap: &[u8], gd: &mut GroupDesc) {
    let desc_size = ext4_sb_get_desc_size::<A>();

    let csum = ext4_balloc_bitmap_csum::<A>(bitmap);
    let lo_csum = (csum & 0xFFFF).to_le();
    let hi_csum = (csum >> 16).to_le();

    let s = read_super_block::<A>();
    if (s.feature_ro_compat & 0x400) >> 10 == 0 {
        return;
    }

    gd.bg_block_bitmap_csum_lo = lo_csum as u16;
    if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
        gd.bg_block_bitmap_csum_hi = hi_csum as u16;
    }
}

pub fn ext4_ialloc_set_bitmap_csum<A:Ext4Traits>(bitmap: &[u8], gd: &mut GroupDesc) {
    let desc_size = ext4_sb_get_desc_size::<A>();

    let csum = ext4_ialloc_bitmap_csum::<A>(bitmap);
    let lo_csum = (csum & 0xFFFF).to_le();
    let hi_csum = (csum >> 16).to_le();

    let s = read_super_block::<A>();
    if (s.feature_ro_compat & 0x400) >> 10 == 0 {
        return;
    }

    gd.bg_inode_bitmap_csum_lo = lo_csum as u16;
    if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
        gd.bg_inode_bitmap_csum_hi = hi_csum as u16;
    }
}

pub fn ext4_crc32c(crc: u32, buf: &[u8], size: u32) -> u32 {
    crc32(crc, buf, size, &CRC32C_TAB)
}

pub const EXT4_CRC32_INIT: u32 = 0xFFFFFFFF;

pub fn ext4_ialloc_bitmap_csum<A:Ext4Traits>(bitmap: &[u8]) -> u32 {
    let s = read_super_block::<A>();
    let mut csum = 0;
    let v = (s.feature_ro_compat & 0x400) >> 10;
    if v == 1 {
        let inodes_per_group = s.inodes_per_group;

        let uuid = s.uuid;

        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);

        csum = ext4_crc32c(csum, bitmap, (inodes_per_group + 7) / 8);
    }

    csum
}

pub fn ext4_balloc_bitmap_csum<A:Ext4Traits>(bitmap: &[u8]) -> u32 {
    let s = read_super_block::<A>();

    let mut csum = 0;

    let v = (s.feature_ro_compat & 0x400) >> 10;

    if v == 1 {
        let blocks_per_group = s.blocks_per_group;

        let uuid = s.uuid;

        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);

        csum = ext4_crc32c(csum, bitmap, (blocks_per_group / 8) as u32);
    }

    csum
}

pub fn ext4_inode_get_csum<A: Ext4Traits>(inode: &Ext4Inode) -> u32 {
    let s = read_super_block::<A>();
    let inode_size = s.inode_size;
    let mut v: u32 = inode.osd2.l_i_checksum_lo as u32;

    if inode_size > 128 {
        v |= (inode.i_checksum_hi as u32) << 16;
    }
    v
}

pub fn ext4_fs_inode_checksum<A: Ext4Traits>(inode: &mut Ext4Inode, index: u32) -> u32 {
    let mut checksum = 0;
    let s = read_super_block::<A>();
    let inode_size = s.inode_size;
    let v = (s.feature_ro_compat & 0x400) >> 10;
    if v == 1 {
        let orig_checksum = ext4_inode_get_csum::<A>(inode);

        let ino_index = index as u32;
        let ino_gen = inode.generation;

        // Preparation: temporarily set bg checksum to 0
        inode.ext4_inode_set_csum(0, inode_size as u16);

        let uuid = s.uuid;

        // First calculate crc32 checksum against fs uuid
        checksum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);

        // Then calculate crc32 checksum against inode number
        // and inode generation
        checksum = ext4_crc32c(checksum, &ino_index.to_le_bytes(), 4);

        checksum = ext4_crc32c(checksum, &ino_gen.to_le_bytes(), 4);

        let mut raw_data = [0u8; 0x100];

        copy_inode_to_array(&inode, &mut raw_data);

        checksum = ext4_crc32c(checksum, &raw_data, inode_size as u32);

        inode.ext4_inode_set_csum(orig_checksum, inode_size as u16);

        let origin = ext4_inode_get_csum::<A>(inode);

        if inode_size == 128 {
            checksum &= 0xFFFF;
        }
    }
    checksum
}

pub fn ext4_fs_bg_checksum<A:Ext4Traits>(bgid: u32, bg: &mut GroupDesc) -> u16 {
    let mut crc = 0;

    let s = read_super_block::<A>();

    let desc_size = ext4_sb_get_desc_size::<A>();
    // 只有当文件系统支持校验和时才计算
    let v = (s.feature_ro_compat & 0x400) >> 10;
    if v == 1 {
        let mut orig_checksum = 0;
        let mut checksum = 0;

        (*bg).bg_checksum = 0;

        // 准备：暂时将bg校验和设为0
        orig_checksum = bg.bg_checksum;

        let uuid = s.uuid;
        // 首先计算fs uuid的crc32校验和
        checksum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);

        // 然后计算bgid的crc32校验和
        checksum = ext4_crc32c(checksum, &bgid.to_le_bytes(), 4);

        let mut raw_data = [0u8; 0x40];
        copy_block_group_to_array(&bg, &mut raw_data, 0);

        // 最后计算block_group_desc的crc32校验和
        checksum = ext4_crc32c(checksum, &raw_data, desc_size as u32);
        (*bg).bg_checksum = orig_checksum;

        crc = (checksum & 0xFFFF) as u16;
    }

    println!("right ext4_fs_bg_checksum=crc {:x}", crc);
    crc
}

pub fn ext4_fs_verify_bg_csum<A:Ext4Traits>(bgid: u32, bg: &mut GroupDesc) -> bool {
    let check_sum = bg.bg_checksum;

    let crc = ext4_fs_bg_checksum::<A>(bgid, bg);
    println!(
        "ext4_fs_verify_bg_csum crc {:x?}  checksum {:x?}",
        crc, check_sum
    );
    if check_sum == crc {
        return true;
    }
    false
}

pub fn ext4_ialloc_verify_bitmap_csum<A: Ext4Traits>(bg: &GroupDesc, bitmap: &[u8]) -> bool {
    let s: Ext4SuperBlock = read_super_block::<A>();
    let desc_size = ext4_sb_get_desc_size::<A>();
    let csum = ext4_ialloc_bitmap_csum::<A>(bitmap);
    let lo_csum = (csum & 0xFFFF).to_le();
    let hi_csum = (csum >> 16).to_le();

    if (s.feature_ro_compat & 0x400) == 0 {
        return true;
    }

    if bg.bg_inode_bitmap_csum_lo != lo_csum as u16 {
        return false;
    }

    if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
        if bg.bg_inode_bitmap_csum_hi != hi_csum as u16 {
            return false;
        }
    }

    true
}

// 定义ext4_balloc_verify_bitmap_csum这个函数
pub fn ext4_balloc_verify_bitmap_csum<A:Ext4Traits>(bg: &GroupDesc, bitmap: &[u8]) -> bool {
    let s = read_super_block::<A>();
    let desc_size = ext4_sb_get_desc_size::<A>();
    let checksum = ext4_balloc_bitmap_csum::<A>(bitmap);
    let lo_checksum = (checksum & 0xFFFF).to_le();
    let hi_checksum = (checksum >> 16).to_le();

    // 如果不支持校验和，返回true
    let v = (s.feature_ro_compat & 0x400) >> 10;
    if v == 0 {
        return true;
    }

    // 如果低16位校验和不匹配，返回false
    if bg.bg_block_bitmap_csum_lo != lo_checksum as u16 {
        return false;
    }

    // 如果描述符大小是最大值，还要检查高16位校验和
    if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
        if bg.bg_block_bitmap_csum_hi != hi_checksum as u16 {
            return false;
        }
    }

    // 如果都匹配，返回true
    true
}

pub fn ext4_fs_verify_inode_csum<A: Ext4Traits>(inode: &mut Ext4Inode, index: u32) -> bool {
    let csum = ext4_inode_get_csum::<A>(inode);

    let verify = ext4_fs_inode_checksum::<A>(inode, index);
    // let verify = ext4_inode_csum(inode, index);

    println!("inode actuall csum {:x?} verfiy {:x?}", csum, verify);
    return verify == csum;
}

pub fn ext4_fs_set_inode_checksum<A:Ext4Traits>(inode: &mut Ext4Inode, index: u32) {
    let super_block = read_super_block::<A>();
    let inode_size = super_block.inode_size;

    let csum = ext4_fs_inode_checksum::<A>(inode, index);
    // let csum = ext4_inode_csum(inode, index);

    inode.ext4_inode_set_csum(csum, inode_size);
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct Ext4Foo {
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
    pub faddr: u32,

    pub l_i_blocks_high: u16, // 原来是l_i_reserved1
    pub l_i_file_acl_high: u16,
    pub l_i_uid_high: u16,    // 这两个字段
    pub l_i_gid_high: u16,    // 原来是reserved2[0]
    pub l_i_checksum_lo: u16, // crc32c(uuid+inum+inode) LE
    pub l_i_reserved: u16,

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
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntryTail {
    pub reserved_zero1: u32, // 假装未使用
    pub rec_len: u16,        // 12
    pub reserved_zero2: u8,  // 零文件名长度
    pub reserved_ft: u8,     // 0xDE，假文件类型
    pub checksum: u32,       // crc32c(uuid+inum+dirblock)
}

// 函数定义，用于获取目录块的尾部校验和项
pub fn ext4_dirent_tail(block: &[u8], blocksize: usize) -> Ext4DirEntryTail {
    unsafe {
        let ptr = block as *const [u8] as *mut u8;
        *(ptr.add(blocksize - core::mem::size_of::<Ext4DirEntryTail>()) as *mut Ext4DirEntryTail)
    }
}

// 遍历目录块，找到尾部的校验和项
pub fn ext4_dir_get_tail<A: Ext4Traits>(inode_blocks: &[u8]) -> Option<Ext4DirEntryTail> {
    let block_size = ext4_sb_get_block_size::<A>();
    println!("block_size {:x?}", block_size);
    let t = ext4_dirent_tail(inode_blocks, block_size as usize);

    if t.reserved_zero1 != 0 || t.reserved_zero2 != 0 {
        return None;
    }
    if t.rec_len.to_le() != core::mem::size_of::<Ext4DirEntryTail>() as u16 {
        return None;
    }
    if t.reserved_ft != 0xDE {
        return None;
    }
    Some(t)
}

pub const EXT_MAX_BLOCKS: ext4_lblk_t = core::u32::MAX;

// 从给定的路径中返回下一个已分配的块
pub fn ext4_ext_next_allocated_block(path: &ext4_extent_path) -> ext4_lblk_t {
    EXT_MAX_BLOCKS
}

pub fn ext4_fwrite<A: Ext4Traits>(ext4_file: &mut Ext4File, data: &[u8], size: u64) {
    let super_block = read_super_block::<A>();
    let inode = read_inode::<A>(ext4_file.inode as u64, &super_block);

    let block_size = ext4_sb_get_block_size::<A>() as u64;
    let iblock_last = ext4_file.fpos + size / block_size;
    let mut iblk_idx = ext4_file.fpos / block_size;
    let ifile_blocks = ext4_file.fsize + block_size - 1 / block_size;

    println!("iblock_last {:x?}", iblock_last);
    println!("iblk_idx{:x?}", iblk_idx);

    println!("\n\n\n\n\n\n\n\n\n\n");

    let mut fblk = 0;

    let mut fblock_start = 0;
    let mut fblock_count = 0;

    let mut size = 8192;

    let mut inode_ref = Ext4InodeRef {
        inode: &inode,
        index: ext4_file.inode,
        dirty: false,
    };

    let mut write_size = 0;

    while size >= block_size {
        // while iblk_idx < iblock_last {
        while iblk_idx < iblock_last {
            // if iblk_idx < ifile_blocks{

            if iblk_idx < ifile_blocks {
                println!("\n\n\n\n");

                println!(
                    "inode num {:x?} inode_ref block={:x?}",
                    inode_ref.index, inode_ref.inode.block
                );
                ext4_fs_append_inode_dblk_new::<A>(&mut inode_ref, iblk_idx as u32, &mut fblk);
            }

            iblk_idx += 1;

            if fblock_start == 0 {
                fblock_start = fblk;
            }

            fblock_count += 1;
        }

        size -= block_size;

        println!("fblock_count {:x?}", fblock_count);
    }

    // write_size = (BLOCK_SIZE * fblock_count)  as usize;
    println!("fblk {:x?}", fblk);
    println!("write size {:x?}", write_size);
    println!(" inode blocks {:x?}", inode.blocks);

    for i in 0..fblock_count {
        let idx = i * BLOCK_SIZE as usize;
        let offset = ((fblk + i as u64) * BLOCK_SIZE) as u64;
        A::write_block(offset, &data[idx..(idx + BLOCK_SIZE as usize)]);

        let write_data = A::read_block(offset);
        println!("write data {:x?}", &write_data[..10]);
    }
}

pub fn ext4_bmap_bit_find_clr(bmap: &[u8], sbit: u32, ebit: u32, bit_id: &mut u32) -> bool {
    let mut i: u32;
    let mut bcnt = ebit - sbit;

    i = sbit;

    while i & 7 != 0 {
        if bcnt == 0 {
            return false;
        }

        if ext4_bmap_is_bit_clr(bmap, i) {
            *bit_id = sbit;
            return true;
        }

        i += 1;
        bcnt -= 1;
    }

    let mut sbit = i;
    let mut bmap = &bmap[(sbit >> 3) as usize..];
    while bcnt >= 8 {
        if bmap[0] != 0xFF {
            for i in 0..8 {
                if ext4_bmap_is_bit_clr(bmap, i) {
                    *bit_id = sbit + i;
                    return true;
                }
            }
        }

        bmap = &bmap[1..];
        bcnt -= 8;
        sbit += 8;
    }

    for i in 0..bcnt {
        if ext4_bmap_is_bit_clr(bmap, i) {
            *bit_id = sbit + i;
            return true;
        }
    }

    false
}

