#![feature(error_in_core)]

pub mod consts;
pub mod ext4_error;
pub mod ext4;
pub mod prelude;
pub mod ext4_defs;
pub mod utils;

pub use consts::*;
pub use ext4_error::*;
pub use ext4::*;
pub use prelude::*;
pub use ext4_defs::*;
pub use utils::*;

extern crate alloc;

#[cfg(test)]
mod tests {
    mod write_test {

        #[test]
        fn test_write() {}
    }
}
