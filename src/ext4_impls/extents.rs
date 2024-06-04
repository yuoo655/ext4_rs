use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
    /// Find an extent in the extent tree.
    /// 
    /// Parms:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    /// 
    /// Returns:
    /// Result<SearchPath> - search path
    /// 
    /// 如果 depth > 0，则查找extent_index，查找目标 lblock 对应的 extent。
    /// 如果 depth = 0，则直接在root节点中查找 extent，查找目标 lblock 对应的 extent。
    pub fn find_extent(&self, inode_ref: &Ext4InodeRef, lblock: Ext4Lblk) -> Result<SearchPath> {
        let mut search_path = SearchPath::new();

        // Load the root node
        let root_data: &[u8; 60] =
            unsafe { core::mem::transmute::<&[u32; 15], &[u8; 60]>(&inode_ref.inode.block) };
        let mut node = ExtentNode::load_from_data(root_data, true).unwrap();

        let mut depth = node.header.depth;

        // Traverse down the tree if depth > 0
        while depth > 0 {
            let index_pos = node.binsearch_idx(lblock);
            if let Some(pos) = index_pos {
                let index = node.get_index(pos)?;
                let next_block = index.leaf_lo;

                search_path.path.push(ExtentPathNode {
                    header: node.header.clone(),
                    index: Some(index),
                    extent: None,
                    position: pos,
                    pblock: next_block as u64,
                });

                let next_block = search_path.path.last().unwrap().index.unwrap().leaf_lo;
                let next_data = self
                    .block_device
                    .read_offset(next_block as usize * BLOCK_SIZE);
                node = ExtentNode::load_from_data(&next_data, false)?;
                depth -= 1;
            } else {
                return_errno_with_message!(Errno::ENOENT, "Extentindex not found");
            }
        }

        // Handle the case where depth is 0 (root node)
        if let Some((extent, pos)) = node.binsearch_extent(lblock) {
            search_path.path.push(ExtentPathNode {
                header: node.header.clone(),
                index: None,
                extent: Some(extent),
                position: pos,
                pblock:extent.start_pblock(),
            });
            search_path.depth = node.header.depth;
            search_path.maxdepth = node.header.depth;

            Ok(search_path)
        } else {
            return_errno_with_message!(Errno::ENOENT, "Extent not found");
        }
    }

    /// Insert an extent into the extent tree.
    fn insert_extent(&self, inode_ref: &mut Ext4InodeRef, newex: &mut Ext4Extent) -> Result<()> {
        let newex_first_block = newex.first_block;

        let mut search_path = self.find_extent(inode_ref, newex_first_block)?;

        let depth = search_path.depth as usize;
        let node = &mut search_path.path[depth]; // Get the node at the current depth

        let header = node.header.clone();

        // Insert to exsiting extent
        if let Some(mut ex) = node.extent.clone() {
            let pos = node.position;
            let last_extent_pos = header.entries_count as usize - 1;

            // Insert right
            // found_ext:   |<---found_ext--->|         |<---next_extent--->|
            //              10               20         30                40
            // insert:      |<---found_ext--->|<---newex---><---next_extent--->|
            //              10               20            30                40
            // merge:       |<---found_ext--->|<---newex--->|
            //              10               20            40
            if pos < last_extent_pos
                && ((ex.first_block + ex.block_count as u32) < newex.first_block)
            {
                if let Some(next_extent) = self.get_extent_from_node(node, pos + 1) {
                    if self.can_merge(&next_extent, &newex) {
                        self.merge_extent(&mut search_path, newex, &next_extent)?;
                        return Ok(());
                    }
                }
            }

            // Insert left
            //  found_ext:  |<---found_ext--->|         |<---ext2--->|
            //              20              30         40          50
            // insert:   |<---prev_extent---><---newex--->|<---found_ext--->|....|<---ext2--->|
            //           0                  10          20                 30    40          50
            // merge:    |<---newex--->|<---found_ext--->|....|<---ext2--->|
            //           0            20                30    40          50
            if pos > 0 && (newex.first_block + newex.block_count as u32) < ex.first_block {
                if let Some(mut prev_extent) = self.get_extent_from_node(node, pos - 1) {
                    if self.can_merge(&prev_extent, &newex) {
                        self.merge_extent(&mut search_path, &mut prev_extent, &newex)?;
                        return Ok(());
                    }
                }
            }

            // Try to Insert to found_ext
            // found_ext:   |<---found_ext--->|         |<---ext2--->|
            //              20              30         50          60
            // insert:      |<---found_ext---><---newex--->|         |<---ext2--->|
            //              20              30            40         50          60
            // merge:       |<---newex--->|      |<---ext2--->|
            //              20           40      50          60
            if self.can_merge(&ex, &newex) {
                self.merge_extent(&mut search_path, &mut ex, &newex)?;
                return Ok(());
            }
        }

        // Check if there's space to insert the new extent
        //                full         full
        // Before:   |<---ext1--->|<---ext2--->|
        //           10           20          30

        //                full          full
        // insert:   |<---ext1--->|<---ext2--->|<---newex--->|
        //           10           20           30           35
        if header.entries_count < header.max_entries_count {
            self.insert_new_extent(&mut search_path, newex)?;
        } else {
            // Create a new leaf node
            self.create_new_leaf(inode_ref, &mut search_path, newex)?;
        }

        Ok(())
    }

    /// Get extent from the node at the given position.
    fn get_extent_from_node(&self, node: &ExtentPathNode, pos: usize) -> Option<Ext4Extent> {
        let data = self
            .block_device
            .read_offset(node.pblock as usize * BLOCK_SIZE);
        let extent_node = ExtentNode::load_from_data(&data, false).unwrap();

        extent_node.get_extent(pos)
    }

    /// Check if two extents can be merged.
    fn can_merge(&self, ex1: &Ext4Extent, ex2: &Ext4Extent) -> bool {
        // Check if the extents have the same unwritten state
        if ex1.is_unwritten() != ex2.is_unwritten() {
            return false;
        }

        // Check if the block ranges are contiguous
        if ex1.first_block + ex1.block_count as u32 != ex2.first_block {
            return false;
        }

        // Check if the merged length would exceed the maximum allowed length
        if ex1.block_count + ex2.block_count > EXT_INIT_MAX_LEN as u16 {
            return false;
        }

        // Check if the merged length would exceed the maximum allowed length for unwritten extents
        if ex1.is_unwritten() && ex1.block_count + ex2.block_count > EXT_UNWRITTEN_MAX_LEN as u16 {
            return false;
        }

        // Check if the physical blocks are contiguous
        if ex1.start_pblock() + ex1.block_count as u64 == ex2.start_pblock() {
            return true;
        }

        false
    }

    fn merge_extent(
        &self,
        search_path: &mut SearchPath,
        left_ext: &mut Ext4Extent,
        right_ext: &Ext4Extent,
    ) -> Result<()> {
        let depth = search_path.depth as usize;
        // Get the node at the current depth
        let node = &mut search_path.path[depth];

        // Ensure that the extents can be merged
        assert!(self.can_merge(&left_ext, &right_ext));

        // Update the length of the left extent to include the right extent
        left_ext.block_count += right_ext.block_count;

        if depth == 0 {
            return Ok(());
        }

        Ok(())
    }

    fn insert_new_extent(
        &self,
        search_path: &mut SearchPath,
        new_extent: &mut Ext4Extent,
    ) -> Result<()> {
        // Implement logic to insert a new extent
        unimplemented!()
    }

    fn create_new_leaf(
        &self,
        inode_ref: &mut Ext4InodeRef,
        search_path: &mut SearchPath,
        new_extent: &mut Ext4Extent,
    ) -> Result<()> {
        // Implement logic to create a new leaf
        unimplemented!()
    }
}
