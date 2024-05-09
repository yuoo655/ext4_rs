#![feature(error_in_core)]
#![no_std]

pub mod consts;
pub mod ext4_error;
pub mod prelude;
pub mod ext4_structs;
pub mod utils;
pub mod ext4_impl;
pub mod ext4_interface;

pub use consts::*;
pub use ext4_error::*;
// pub use ext4::*;
pub use ext4_structs::*;
pub use utils::*;
pub use ext4_interface::*;
#[allow(unused)]
pub use ext4_impl::*;


extern crate alloc;

#[cfg(test)]
mod tests {
    mod write_test {

        #[test]
        fn test_write() {}
    }
}
