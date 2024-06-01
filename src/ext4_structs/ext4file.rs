// use crate::consts::*;
// use crate::BASE_OFFSET;
use crate::prelude::*;
use super::*;
// use core::mem::size_of;
// use super::*;
// use crate::consts::*;
// use crate::prelude::*;
// use crate::utils::*;
// use crate::BLOCK_SIZE;
// use crate::BlockDevice;
// use crate::Ext4;


#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    Start(usize),
    End(isize),
    Current(isize),
}

/// 文件描述符
#[derive(Debug)]
pub struct Ext4File {
    /// 挂载点句柄
    pub mp: *mut Ext4MountPoint,
    /// 文件 inode id
    pub inode: u32,
    /// 打开标志
    pub flags: u32,
    /// 文件大小
    pub fsize: u64,
    /// 实际文件位置
    pub fpos: usize,
}

impl Ext4File {
    pub fn new() -> Self {
        Self {
            mp: core::ptr::null_mut(),
            inode: 0,
            flags: 0,
            fsize: 0,
            fpos: 0,
        }
    }
}