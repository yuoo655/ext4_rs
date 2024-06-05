use crate::prelude::*;
use crate::return_errno_with_message;
use crate::utils::*;

use crate::ext4_defs::*;
impl Ext4 {
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Self {
        // Load the superblock
        let block = Block::load(block_device.clone(), SUPERBLOCK_OFFSET);
        let super_block: Ext4Superblock = block.read_as();

        Ext4 {
            block_device,
            super_block,
        }
    }

    // with dir result search path offset
    pub fn generic_open(
        &self,
        path: &str,
        parent_inode_num: u32,
        create: bool,
        ftype: u16,
        name_off: &mut u32,
    ) -> Result<usize> {
        let mut is_goal = false;

        let mut parent = parent_inode_num;

        let mut search_path = path;

        let mut dir_search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        let mut current_inode_ref = self.get_inode_ref(parent);

        loop {
            while search_path.starts_with('/') {
                *name_off += 1; // Skip the slash
                search_path = &search_path[1..];
            }

            let len = path_check(search_path, &mut is_goal);

            let current_path = &search_path[..len];

            if len == 0 || search_path.is_empty() {
                break;
            }

            search_path = &search_path[len..];

            let r = self.dir_find_entry(parent, current_path, &mut dir_search_result);

            if let Err(e) = r {
                if e.error() != Errno::ENOENT.into() || !create {
                    return_errno_with_message!(Errno::ENOENT, "No such file or directory");
                }

                let mut inode_mode = 0;
                if is_goal {
                    inode_mode = ftype;
                } else {
                    inode_mode = InodeFileType::S_IFDIR.bits();
                }

                let new_inode_ref = self.create(parent, current_path, inode_mode)?;

                current_inode_ref = new_inode_ref;
                parent = current_inode_ref.inode_num;

                continue;
            }
            

            if is_goal {
                break;
            }
            *name_off += len as u32;
        }

        if is_goal {
            return Ok(EOK);
        }
        Ok(EOK)
    }

    #[allow(unused)]
    pub fn dir_mk(&self, path: &str) -> Result<usize> {
        let mut nameoff = 0;

        let filetype = InodeFileType::S_IFDIR;
        let r = self.generic_open(path, ROOT_INODE, true, filetype.bits(), &mut nameoff);
        r
    }
}
