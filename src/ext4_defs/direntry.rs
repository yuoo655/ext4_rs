use crate::prelude::*;
use crate::utils::*;

use super::*;

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
