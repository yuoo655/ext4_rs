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
    /// Kind of file (directory, file, pipe, etc)
    pub kind: InodeFileType,
    /// Permissions
    pub perm: u16,
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
            kind: inode.file_type(),
            perm: inode.file_perm().bits(), // Extract permission bits
            nlink: inode.links_count() as u32,
            uid: inode.uid() as u32,
            gid: inode.gid() as u32,
            rdev: inode.faddr(),
            blksize: BLOCK_SIZE as u32,
            flags: inode.flags(),
        }
    }
}
