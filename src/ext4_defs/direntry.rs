use crate::prelude::*;
use crate::utils::*;

use super::*;

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

/// Directory entry structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntry {
    pub inode: u32,               // 该目录项指向的inode的编号
    pub entry_len: u16,           // 到下一个目录项的距离
    pub name_len: u8,             // 低8位的文件名长度
    pub inner: Ext4DirEnInternal, // 联合体成员
    pub name: [u8; 255],          // 文件名
}


/// Internal directory entry structure.
#[repr(C)]
#[derive(Clone, Copy)]
pub union Ext4DirEnInternal {
    pub name_length_high: u8, // 高8位的文件名长度
    pub inode_type: u8,       // 引用的inode的类型（在rev >= 0.5中）
}


/// Fake directory entry structure. Used for directory entry iteration.
#[repr(C)]
pub struct Ext4FakeDirEntry {
    inode: u32,
    entry_length: u16,
    name_length: u8,
    inode_type: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntryTail {
    pub reserved_zero1: u32,
    pub rec_len: u16,
    pub reserved_zero2: u8,
    pub reserved_ft: u8,
    pub checksum: u32, // crc32c(uuid+inum+dirblock)
}

pub struct Ext4DirSearchResult{
    pub dentry: Ext4DirEntry, 
    pub pblock_id: usize, // disk block id
    pub offset: usize, // offset in block
    pub prev_offset: usize, //prev direntry offset
}


impl Ext4DirSearchResult {
    pub fn new(dentry: Ext4DirEntry) -> Self {
        Self {
            dentry,
            pblock_id: 0,
            offset: 0,
            prev_offset: 0,
        }
    }
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

/// Directory entry implementation.
impl Ext4DirEntry {

    /// Check if the directory entry is unused.
    pub fn unused(&self) -> bool {
        self.inode == 0
    }

    /// Set the directory entry as unused.
    pub fn set_unused(&mut self) {
        self.inode = 0
    }

    /// Check name
    pub fn compare_name(&self, name: &str) -> bool {
        if self.name_len as usize == name.len(){
            return &self.name[..name.len()] == name.as_bytes()
        }
        false
    }

    /// Entry length
    pub fn entry_len(&self) -> u16 {
        self.entry_len
    }

    /// Dir type
    pub fn get_de_type(&self) -> u8 {
        let de_type = unsafe { self.inner.inode_type } as u8;
        de_type
    }

    /// Get name to string
    pub fn get_name(&self) -> String {
        let name_len = self.name_len as usize;
        let name = &self.name[..name_len];
        let name = core::str::from_utf8(name).unwrap();
        name.to_string()
    }

    /// Get name len
    pub fn get_name_len(&self) -> usize {
        let name_len = self.name_len as usize;
        name_len
    }

    /// 计算目录项的实际使用长度（不包括填充字节）
    pub fn actual_len(&self) -> usize {
        size_of::<Ext4FakeDirEntry>() + self.name_len as usize
    }


    /// 计算对齐后的目录项长度（包括填充字节）
    pub fn used_len_aligned(&self) -> usize {
        let mut len = self.actual_len();
        if len % 4 != 0 {
            len += 4 - (len % 4);
        }
        len
    }

    
    pub fn write_entry(&mut self, entry_len: u16, inode: u32, name: &str, de_type:DirEntryType) {
        self.inode = inode;
        self.entry_len = entry_len;
        self.name_len = name.len() as u8;
        self.inner.inode_type = de_type.bits();
        self.name[..name.len()].copy_from_slice(name.as_bytes());
    }

}

impl Ext4DirEntry {

    /// Get the checksum of the directory entry.
    #[allow(unused)]
    pub fn ext4_dir_get_csum(&self, s: &Ext4Superblock, blk_data: &[u8], ino_gen: u32) -> u32 {
        let ino_index = self.inode;

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

    /// Write de to block
    pub fn write_de_to_blk(&self, dst_blk: &mut Block, offset: usize) {
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        let data = unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, count) };
        dst_blk.data.splice(
            offset..offset + core::mem::size_of::<Ext4DirEntry>(),
            data.iter().cloned(),
        );
        // assert_eq!(dst_blk.block_data[offset..offset + core::mem::size_of::<Ext4DirEntry>()], data[..]);
    }

    /// Copy the directory entry to a slice.
    pub fn copy_to_slice(&self, array: &mut [u8], offset: usize) {
        let de_ptr = self as *const Ext4DirEntry as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        unsafe {
            core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
        }
    }
}


impl Ext4DirEntryTail{
    pub fn new() -> Self {
        Self {
            reserved_zero1: 0,
            rec_len: size_of::<Ext4DirEntryTail>() as u16,
            reserved_zero2: 0,
            reserved_ft: 0xDE,
            checksum: 0,
        }
    }

    pub fn tail_set_csum(
        &mut self,
        s: &Ext4Superblock,
        diren: &Ext4DirEntry,
        blk_data: &[u8],
        ino_gen: u32,
    ) {
        let csum = diren.ext4_dir_get_csum(s, blk_data, ino_gen);
        self.checksum = csum;
    }

    pub fn copy_to_slice(&self, array: &mut [u8]) {
        unsafe {
        let offset = BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>();
        let de_ptr = self as *const Ext4DirEntryTail as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntryTail>();
            core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
        }
    }
}