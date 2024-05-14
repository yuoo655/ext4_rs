// SPDX-License-Identifier: MPL-2.0

/// Ext4Error number.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Errnum {
    EPERM     = 1,     /* Operation not permitted */
    ENOENT    = 2,    /* No such file or directory */
    EIO       = 5,     /* I/O error */
    ENXIO     = 6,     /* No such device or address */
    E2BIG     = 7,     /* Argument list too long */
    ENOMEM    = 12,    /* Out of memory */
    EACCES    = 13,    /* Permission denied */
    EFAULT    = 14,    /* Bad address */
    EEXIST    = 17,    /* File exists */
    ENODEV    = 19,    /* No such device */
    ENOTDIR   = 20,    /* Not a directory */
    EISDIR    = 21,    /* Is a directory */
    EINVAL    = 22,    /* Invalid argument */
    EFBIG     = 27,     /* File too large */
    ENOSPC    = 28,    /* No space left on device */
    EROFS     = 30,     /* Read-only file system */
    EMLINK    = 31,    /* Too many links */
    ERANGE    = 34,    /* Math result not representable */
    ENOTEMPTY = 39,    /* Directory not empty */
    ENODATA   = 61,   /* No data available */
    ENOTSUP   = 95,   /* Not supported */
    ELINKFIAL = 97,   /* Link failed */
    EALLOCFIAL= 98,   /* Inode alloc failed */
}

/// error used in this crate
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(unused)]
pub struct Ext4Error {
    errno: Errnum,
    msg: Option<&'static str>,
}

impl Ext4Error {
    pub const fn new(errno: Errnum) -> Self {
        Ext4Error { errno, msg: None }
    }

    pub const fn with_message(errno: Errnum, msg: &'static str) -> Self {
        Ext4Error {
            errno,
            msg: Some(msg),
        }
    }

    pub const fn error(&self) -> Errnum {
        self.errno
    }
}

impl From<Errnum> for Ext4Error {
    fn from(errno: Errnum) -> Self {
        Ext4Error::new(errno)
    }
}


impl From<core::str::Utf8Error> for Ext4Error {
    fn from(_: core::str::Utf8Error) -> Self {
        Ext4Error::with_message(Errnum::EINVAL, "Invalid utf-8 string")
    }
}

impl From<alloc::string::FromUtf8Error> for Ext4Error {
    fn from(_: alloc::string::FromUtf8Error) -> Self {
        Ext4Error::with_message(Errnum::EINVAL, "Invalid utf-8 string")
    }
}

impl From<core::ffi::FromBytesUntilNulError> for Ext4Error {
    fn from(_: core::ffi::FromBytesUntilNulError) -> Self {
        Ext4Error::with_message(Errnum::E2BIG, "Cannot find null in cstring")
    }
}

impl From<core::ffi::FromBytesWithNulError> for Ext4Error {
    fn from(_: core::ffi::FromBytesWithNulError) -> Self {
        Ext4Error::with_message(Errnum::E2BIG, "Cannot find null in cstring")
    }
}


impl From<alloc::ffi::NulError> for Ext4Error {
    fn from(_: alloc::ffi::NulError) -> Self {
        Ext4Error::with_message(Errnum::E2BIG, "Cannot find null in cstring")
    }
}


#[macro_export]
macro_rules! return_errno {
    ($errno: expr) => {
        return Err($crate::error::Ext4Error::new($errno))
    };
}

#[macro_export]
macro_rules! return_errno_with_message {
    ($errno: expr, $message: expr) => {
        return Err(Ext4Error::with_message($errno, $message))
    };
}
