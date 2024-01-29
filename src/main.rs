extern crate alloc;

use alloc::string;
use alloc::vec;
use bitflags::Flags;
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;
use core::*;

mod prelude;
mod defs;
mod ext4;

use crate::prelude::*;
use crate::defs::*;
use crate::ext4::*;


// use lock::Mutex;
// use lock::MutexGuard;


pub fn main() {
    let f = Ext4FileNew::new();
    let path = "test.txt";
    let r = ext4_fopen(&f, path, "wb");
}
