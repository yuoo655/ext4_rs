# 磁盘布局
![img](ext4-disk-layout.jpg)
EXT4文件系统主要使用块组0中的超级块和块组描述符表，在其他一些特定块组中有超级块和块组描述符表的冗余备份。如果块组中不含冗余备份，那么块组就会以数据块位图开始。当格式化磁盘成为Ext4文件系统的时候，mkfs将在块组描述符表后面分配预留GDT表数据块（“Reserve GDT blocks”）以用于将来扩展文件系统。紧接在预留GDT表数据块后的是数据块位图与inode的表位图，这两个位图分别表示本块组内的数据块与inode的表的使用，索引节点表数据块之后就是存储文件的数据块了。在这些各种各样的块中，超级块，GDT，块位图，索引节点位图都是整个文件系统的元数据，当然的inode表也是文件系统的元数据，但是i节点表是与文件一一对应的，我更倾向于将索引节点当做文件的元数据，因为在实际格式化文件系统的时候，除了已经使用的十来个外，其他的inode表中实际上是没有任何数据 的，直到创建了相应的文件才会分配的inode表，文件系统才会在索引节点表中写入与文件相关的inode的信息。

# meta data

meta data记录了 ext4文件系统的基本信息，包括文件系统的大小、块大小、块组信息、inode信息、挂载时间等等。任何对meta data进行修改的操作,都需要进行crc校验,确保数据的完整性.

![img](checksum.png)

# 超级块
超级块记录整个文件系统的大量信息，如数据块个数、inode个数、支持的特性、管理信息，等待。

# 特殊inodes
EXT4预留了一些索引节点做特殊特性使用，见下表：

表1 Ext4的特殊inode

Inode号用途

0不存在0号inode

1损坏数据块链表

2根目录

3 ACL索引

4 ACL数据

5引导装载程序

6未删除的目录

7预留的块组描述符inode

8日志inode

11第一个非预留的inode，通常是lost + found目录

## 块组描述符
块组描述符表记录了每个块组的元数据的位置，如块位图、inode位图、inode表、数据块的起始位置等等。

# inode位图
块位图记录了块组中的inode的使用情况，如果inode位图中的某个位为1，则表示该inode已经被使用，否则为0。

# 块位图
块位图记录了块组中的数据块的使用情况，如果块位图中的某个位为1，则表示该块已经被使用，否则为0。

# inode
inode是文件系统中的一种数据结构,用于存储文件的元数据信息,包括文件的大小、文件的访问权限、文件的创建时间、文件的修改时间、文件的inode号、文件的数据块等等。inode是文件系统中的一种数据结构,用于存储文件的元数据信息,包括文件的大小、文件的访问权限、文件的创建时间、文件的修改时间、文件的inode号、文件的数据块等等。inode是文件系统中的一种数据结构,用于存储文件的元数据信息,包括文件的大小、文件的访问权限、文件的创建时间、文件的修改时间、文件的inode号、文件的数据块等等。

# 打开文件过程

从挂载点开始ext4_dir_find_entry遍历目录来,对比文件名,找到目标文件，提取direntry中的inode号，这一步也就是查找文件路径到文件inode的过程。

```rust
fn ext4_generic_open(path){
    
    loop {
        ext4_dir_find_entry(path)
    }

}
```

# 读文件

由于ext4默认所有文件都使用extent。extent记录了文件逻辑块号对应磁盘存储的物理块号。读取文件的过程就是寻找文件所有extent的过程。找到extent之后便可从extent中获取物理块号读出数据

```rust
pub fn ext4_file_read<A: Ext4Traits>(ext4_file: &mut Ext4File) {

    // 从inode_data中获取文件的所有extent信息，并存储在extents向量中
    ext4_find_extent::<A>(&inode_data, &mut extents);

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
}
```

# 创建文件

创建文件首先需要分配inode
```rust
let idx = ext4_inode_alloc::<Hal>(1);
```

寻找inode位图找到第一个可用的位
```rust
ext4_bmap_bit_find_clr(data, 0, inodes_in_bg, &mut idx_in_bg);
ext4_bmap_bit_set(&mut raw_data, idx_in_bg);
```

设置相应的inode计数
```rust
ext4_bg_set_free_inodes_count::<A>(&mut gd, free_inodes);

/* Decrease unused inodes count */
ext4_bg_set_itable_unused::<A>(&mut gd, unused as u16);
```

init inode设置inode基础信息
```rust
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
    inode_ref.ext4_inode_set_size(0);
    inode_ref.ext4_inode_set_access_time(0);
    inode_ref.ext4_inode_set_change_inode_time(0);
    inode_ref.ext4_inode_set_modif_time(0);
    inode_ref.ext4_inode_set_del_time(0);
    inode_ref.ext4_inode_set_blocks_count(0);
    inode_ref.ext4_inode_set_flags(0);
    inode_ref.ext4_inode_set_generation(0);
}
```

init inode extent 信息
```rust
pub fn ext4_extent_tree_init(inode_ref: &mut Ext4Inode) {
    /* Initialize extent root header */
    let mut header = unsafe { *ext4_inode_get_extent_header(inode_ref) };
    ext4_extent_header_set_depth(&mut header, 0);
    ext4_extent_header_set_entries_count(&mut header, 0);
    ext4_extent_header_set_generation(&mut header, 0);
    ext4_extent_header_set_magic(&mut header, EXT4_EXTENT_MAGIC);
    ext4_extent_header_set_max_entries_count(&mut header, 4 as u16);
}
```

再接着link inode号到文件名,目录项，父目录，首先找到父目录的目录项，再把当前文件的目录项添加到父目录项的尾部。
```rust
ext4_link::<Hal>(&mp,&root_inode,&mut child_inode_ref,path,name_len,false){

    ext4_dir_find_entry::<A>(&parent_inode, &path, len as u32, &mut dir_search_result);

    /* Add entry to parent directory */
    ext4_dir_add_entry::<A>(parent_inode, child_inode, path, len);

}
```

# 写文件

查找文件逻辑块对应的物理块，如果没有对应物理块则分配一个物理块。
```rust
ext4_fwrite::<Hal>(&mut ext4_file, &write_data, 4096 * 2);

while size >= block_size {
    while iblk_idx < iblock_last {
        if iblk_idx < ifile_blocks {
            ext4_fs_append_inode_dblk_new::<A>(&mut inode_ref, iblk_idx as u32, &mut fblk);
        }
        iblk_idx += 1;
        if fblock_start == 0 {
            fblock_start = fblk;
        }
        fblock_count += 1;
    }
    size -= block_size;
}
```

分配物理块同样要从block bitmap中查询, 当分配完物理块后，就可以填写extent信息了。再把记录了逻辑块和物理块对应信息的extent插入extent树中。最后在相应的物理块中写入数据。
```rust
ext4_balloc_alloc_block()
ext4_ext_insert_extent()
```

# checksum
创建文件，写入文件都涉及对meta data的修改。所有meta data都有crc检验信息。修改元数据后，需设置校验信息，然后写入磁盘
![img](checksum.png)

```rust
例
csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
csum = ext4_crc32c(csum, bitmap, (blocks_per_group / 8) as u32);
```
