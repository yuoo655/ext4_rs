pub mod defs;
pub mod ext4;
pub mod prelude;
pub mod consts;
pub mod utils;

pub use defs::*;
pub use ext4::*;
pub use prelude::*;
pub use consts::*;
pub use utils::*;


extern crate alloc;

#[cfg(test)]
mod tests {
    mod write_test {

        #[test]        
        fn test_write() {
            

        }
    }

}
