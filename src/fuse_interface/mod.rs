use crate::prelude::*;

use crate::ext4_defs::*;

/// fuser interface for ext4
impl Ext4 {
    /// Initialize filesystem.
    /// Called before any other filesystem method.
    /// The kernel module connection can be configured using the KernelConfig object
    fn fuse_init(&mut self) -> Result<usize> {
        Ok(EOK)
    }

    /// Clean up filesystem.
    /// Called on filesystem exit.
    fn fuse_destroy(&mut self) -> Result<usize> {
        Ok(EOK)
    }

    /// Look up a directory entry by name and get its attributes.
    pub fn fuse_lookup(&self, parent: u64, name: &str) -> Result<FileAttr> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        self.dir_find_entry(parent as u32, name, &mut search_result)?;

        let inode_num = search_result.dentry.inode;

        let inode_ref = self.get_inode_ref(inode_num);
        let file_attr = FileAttr::from_inode_ref(&inode_ref);

        Ok(file_attr)
    }

    /// Get file attributes.
    pub fn fuse_getattr(&self, ino: u64) {}

    /// Set file attributes.
    pub fn fuse_setattr(
        &mut self,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<u32>,
        mtime: Option<u32>,
        ctime: Option<u32>,
        fh: Option<u64>,
        crtime: Option<u32>,
        chgtime: Option<u32>,
        bkuptime: Option<u32>,
        flags: Option<u32>,
    ) {
        unimplemented!()
    }

    /// Read symbolic link.
    fn fuse_readlink(&mut self, ino: u64) {}

    /// Create file node.
    /// Create a regular file, character device, block device, fifo or socket node.
    fn fuse_mknod(&mut self, parent: u64, name: &str, mode: u32, umask: u32, rdev: u32) {}
    /// Create a directory.
    fn fuse_mkdir(&mut self, parent: u64, name: &str, mode: u32, umask: u32) {}

    /// Remove a file.
    fn fuse_unlink(&mut self, parent: u64, name: &str) {}

    /// Remove a directory.
    fn fuse_rmdir(&mut self, parent: u64, name: &str) {}

    /// Create a symbolic link.
    fn fuse_symlink(&mut self, parent: u64, link_name: &str, target: &str) {}

    /// Rename a file.
    fn fuse_rename(&mut self, parent: u64, name: &str, newparent: u64, newname: &str, flags: u32) {}

    /// Create a hard link.
    fn fuse_link(&mut self, ino: u64, newparent: u64, newname: &str) {}

    /// Open a file.
    /// Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are
    /// available in flags. Filesystem may store an arbitrary file handle (pointer, index,
    /// etc) in fh, and use this in other all other file operations (read, write, flush,
    /// release, fsync). Filesystem may also implement stateless file I/O and not store
    /// anything in fh. There are also some flags (direct_io, keep_cache) which the
    /// filesystem may set, to change the way the file is opened. See fuse_file_info
    /// structure in <fuse_common.h> for more details.
    fn fuse_open(&mut self, _ino: u64, _flags: i32) {}

    /// Read data.
    /// Read should send exactly the number of bytes requested except on EOF or error,
    /// otherwise the rest of the data will be substituted with zeroes. An exception to
    /// this is when the file has been opened in 'direct_io' mode, in which case the
    /// return value of the read system call will reflect the return value of this
    /// operation. fh will contain the value set by the open method, or will be undefined
    /// if the open method didn't set any value.
    ///
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9
    /// lock_owner: only supported with ABI >= 7.9
    fn fuse_read(
        &mut self,

        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
    ) {
    }

    /// Write data.
    /// Write should return exactly the number of bytes requested except on error. An
    /// exception to this is when the file has been opened in 'direct_io' mode, in
    /// which case the return value of the write system call will reflect the return
    /// value of this operation. fh will contain the value set by the open method, or
    /// will be undefined if the open method didn't set any value.
    ///
    /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set,
    /// the pid, uid, gid, and fh may not match the value that would have been sent if write cachin
    /// is disabled
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9
    /// lock_owner: only supported with ABI >= 7.9
    fn fuse_write(
        &mut self,

        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
    ) {
    }

    /// Flush method.
    /// This is called on each close() of the opened file. Since file descriptors can
    /// be duplicated (dup, dup2, fork), for one open call there may be many flush
    /// calls. Filesystems shouldn't assume that flush will always be called after some
    /// writes, or that if will be called at all. fh will contain the value set by the
    /// open method, or will be undefined if the open method didn't set any value.
    /// NOTE: the name of the method is misleading, since (unlike fsync) the filesystem
    /// is not forced to flush pending writes. One reason to flush data, is if the
    /// filesystem wants to return write errors. If the filesystem supports file locking
    /// operations (setlk, getlk) it should remove all locks belonging to 'lock_owner'.
    fn fuse_flush(&mut self, ino: u64, fh: u64, lock_owner: u64) {}

    /// Release an open file.
    /// Release is called when there are no more references to an open file: all file
    /// descriptors are closed and all memory mappings are unmapped. For every open
    /// call there will be exactly one release call. The filesystem may reply with an
    /// error, but error values are not returned to close() or munmap() which triggered
    /// the release. fh will contain the value set by the open method, or will be undefined
    /// if the open method didn't set any value. flags will contain the same flags as for
    /// open.
    fn fuse_release(
        &mut self,

        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
    ) {
    }

    /// Synchronize file contents.
    /// If the datasync parameter is non-zero, then only the user data should be flushed,
    /// not the meta data.
    fn fuse_fsync(&mut self, ino: u64, fh: u64, datasync: bool) {}

    /// Open a directory.
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in fh, and
    /// use this in other all other directory stream operations (readdir, releasedir,
    /// fsyncdir). Filesystem may also implement stateless directory I/O and not store
    /// anything in fh, though that makes it impossible to implement standard conforming
    /// directory stream operations in case the contents of the directory can change
    /// between opendir and releasedir.
    fn fuse_opendir(&mut self, _ino: u64, _flags: i32) {}

    /// Read directory.
    /// Send a buffer filled using buffer.fill(), with size not exceeding the
    /// requested size. Send an empty buffer on end of stream. fh will contain the
    /// value set by the opendir method, or will be undefined if the opendir method
    /// didn't set any value.
    fn fuse_readdir(&mut self, ino: u64, fh: u64, offset: i64) {}

    /// Read directory.
    /// Send a buffer filled using buffer.fill(), with size not exceeding the
    /// requested size. Send an empty buffer on end of stream. fh will contain the
    /// value set by the opendir method, or will be undefined if the opendir method
    /// didn't set any value.
    fn fuse_readdirplus(&mut self, ino: u64, fh: u64, offset: i64) {}

    /// Release an open directory.
    /// For every opendir call there will be exactly one releasedir call. fh will
    /// contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    fn fuse_releasedir(&mut self, _ino: u64, _fh: u64, _flags: i32) {}

    /// Synchronize directory contents.
    /// If the datasync parameter is set, then only the directory contents should
    /// be flushed, not the meta data. fh will contain the value set by the opendir
    /// method, or will be undefined if the opendir method didn't set any value.
    fn fuse_fsyncdir(&mut self, ino: u64, fh: u64, datasync: bool) {}

    /// Get file system statistics.
    fn fuse_statfs(&mut self, _ino: u64) {}

    /// Set an extended attribute.
    fn fuse_setxattr(&mut self, ino: u64, name: &str, _value: &[u8], flags: i32, position: u32) {}

    /// Get an extended attribute.
    /// If `size` is 0, the size of the value should be sent with `reply.size()`.
    /// If `size` is not 0, and the value fits, send it with `reply.data()`, or
    /// `reply.error(ERANGE)` if it doesn't.
    fn fuse_getxattr(&mut self, ino: u64, name: &str, size: u32) {}

    /// List extended attribute names.
    /// If `size` is 0, the size of the value should be sent with `reply.size()`.
    /// If `size` is not 0, and the value fits, send it with `reply.data()`, or
    /// `reply.error(ERANGE)` if it doesn't.
    fn fuse_listxattr(&mut self, ino: u64, size: u32) {}

    /// Remove an extended attribute.
    fn fuse_removexattr(&mut self, ino: u64, name: &str) {}

    /// Check file access permissions.
    /// This will be called for the access() system call. If the 'default_permissions'
    /// mount option is given, this method is not called. This method is not called
    /// under Linux kernel versions 2.4.x
    fn fuse_access(&mut self, ino: u64, mask: i32) {}

    /// Create and open a file.
    /// If the file does not exist, first create it with the specified mode, and then
    /// open it. Open flags (with the exception of O_NOCTTY) are available in flags.
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in fh,
    /// and use this in other all other file operations (read, write, flush, release,
    /// fsync). There are also some flags (direct_io, keep_cache) which the
    /// filesystem may set, to change the way the file is opened. See fuse_file_info
    /// structure in <fuse_common.h> for more details. If this method is not
    /// implemented or under Linux kernel versions earlier than 2.6.15, the mknod()
    /// and open() methods will be called instead.
    fn fuse_create(&mut self, parent: u64, name: &str, mode: u32, umask: u32, flags: i32) {}

    /// Test for a POSIX file lock.
    fn fuse_getlk(
        &mut self,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
    ) {
    }

    /// Acquire, modify or release a POSIX file lock.
    /// For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but
    /// otherwise this is not always the case.  For checking lock ownership,
    /// 'fi->owner' must be used. The l_pid field in 'struct flock' should only be
    /// used to fill in this field in getlk(). Note: if the locking methods are not
    /// implemented, the kernel will still allow file locking to work locally.
    /// Hence these are only interesting for network filesystems and similar.
    fn fuse_setlk(
        &mut self,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
    ) {
    }

    /// Map block index within file to block index within device.
    /// Note: This makes sense only for block device backed filesystems mounted
    /// with the 'blkdev' option
    fn fuse_bmap(&mut self, ino: u64, blocksize: u32, idx: u64) {}

    /// control device
    fn fuse_ioctl(
        &mut self,
        ino: u64,
        fh: u64,
        flags: u32,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) {
    }

    /// Poll for events
    // #[cfg(feature = "abi-7-11")]
    // fn fuse_poll(
    //     &mut self,
    //     ino: u64,
    //     fh: u64,
    //     kh: u64,
    //     events: u32,
    //     flags: u32,
    // ) {
    // }

    /// Preallocate or deallocate space to a file
    fn fuse_fallocate(&mut self, ino: u64, fh: u64, offset: i64, length: i64, mode: i32) {}

    /// Reposition read/write file offset
    fn fuse_lseek(&mut self, ino: u64, fh: u64, offset: i64, whence: i32) {}

    /// Copy the specified range from the source inode to the destination inode
    fn fuse_copy_file_range(
        &mut self,
        ino_in: u64,
        fh_in: u64,
        offset_in: i64,
        ino_out: u64,
        fh_out: u64,
        offset_out: i64,
        len: u64,
        flags: u32,
    ) {
    }
}
