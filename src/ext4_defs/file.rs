use core::default;

use super::*;


pub struct FileAttr {
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: u32,
    /// Time of last modification
    pub mtime: u32,
    /// Time of last change
    pub ctime: u32,
    /// Time of creation (macOS only)
    pub crtime: u32,
    /// Time of last status change
    pub chgtime: u32,
    /// Backup time (macOS only)
    pub bkuptime: u32,
    /// Kind of file (directory, file, pipe, etc)
    pub kind: InodeFileType,
    /// Permissions
    pub perm: InodePerm,
    /// Number of hard links
    pub nlink: u32,
    /// User id
    pub uid: u32,
    /// Group id
    pub gid: u32,
    /// Rdev
    pub rdev: u32,
    /// Block size
    pub blksize: u32,
    /// Flags (macOS only, see chflags(2))
    pub flags: u32,
}

impl Default for FileAttr {
    fn default() -> Self {
        FileAttr {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
            chgtime: 0,
            bkuptime: 0,
            kind: InodeFileType::S_IFREG,
            perm: InodePerm::S_IREAD | InodePerm::S_IWRITE | InodePerm::S_IEXEC,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
        }
    }
    
}

impl FileAttr {
    pub fn from_inode_ref(inode_ref: &Ext4InodeRef) -> FileAttr {
        let inode_num = inode_ref.inode_num;
        let inode = inode_ref.inode;
        FileAttr {
            ino: inode_num as u64,
            size: inode.size(),
            blocks: inode.blocks_count(),
            atime: inode.atime(),
            mtime: inode.mtime(),
            ctime: inode.ctime(),
            crtime: inode.i_crtime(),
            // todo: chgtime, bkuptime
            chgtime: 0,
            bkuptime: 0,
            kind: inode.file_type(),
            perm: inode.file_perm(), // Extract permission bits
            nlink: inode.links_count() as u32,
            uid: inode.uid() as u32,
            gid: inode.gid() as u32,
            rdev: inode.faddr(),
            blksize: BLOCK_SIZE as u32,
            flags: inode.flags(),
        }
    }
}
