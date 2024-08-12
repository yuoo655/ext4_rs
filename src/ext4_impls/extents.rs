use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
    /// Find an extent in the extent tree.
    ///
    /// Params:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<SearchPath>` - search path
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
        let mut pblock_of_node = 0;
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
                    pblock_of_node: pblock_of_node,
                });

                let next_block = search_path.path.last().unwrap().index.unwrap().leaf_lo;
                let mut next_data = self
                    .block_device
                    .read_offset(next_block as usize * BLOCK_SIZE);
                node = ExtentNode::load_from_data_mut(&mut next_data, false)?;
                depth -= 1;
                search_path.depth += 1;
                pblock_of_node = next_block as usize;
            } else {
                return_errno_with_message!(Errno::ENOENT, "Extentindex not found");
            }
        }

        // Handle the case where depth is 0
        if let Some((extent, pos)) = node.binsearch_extent(lblock) {
            search_path.path.push(ExtentPathNode {
                header: node.header.clone(),
                index: None,
                extent: Some(extent),
                position: pos,
                pblock: extent.get_pblock(),
                pblock_of_node: pblock_of_node,
            });
            search_path.maxdepth = node.header.depth;

            Ok(search_path)
        } else {
            search_path.path.push(ExtentPathNode {
                header: node.header.clone(),
                index: None,
                extent: None,
                position: 0,
                pblock: 0,
                pblock_of_node: pblock_of_node,
            });
            return Ok(search_path);
        }
    }

    /// Insert an extent into the extent tree.
    pub fn insert_extent(
        &self,
        inode_ref: &mut Ext4InodeRef,
        newex: &mut Ext4Extent,
    ) -> Result<()> {
        let newex_first_block = newex.first_block;

        let mut search_path = self.find_extent(inode_ref, newex_first_block)?;

        let depth = search_path.depth as usize;
        let node = &search_path.path[depth]; // Get the node at the current depth

        let at_root = node.pblock_of_node == 0;
        let header = node.header.clone();

        // Node is empty (no extents)
        if header.entries_count == 0 {
            // If the node is empty, insert the new extent directly
            self.insert_new_extent(inode_ref, &mut search_path, newex)?;
            return Ok(());
        }

        // Insert to exsiting extent
        if let Some(mut ex) = node.extent {
            let pos = node.position;
            let last_extent_pos = header.entries_count as usize - 1;

            // Try to Insert to found_ext
            // found_ext:   |<---found_ext--->|         |<---ext2--->|
            //              20              30         50          60
            // insert:      |<---found_ext---><---newex--->|         |<---ext2--->|
            //              20              30            40         50          60
            // merge:       |<---newex--->|      |<---ext2--->|
            //              20           40      50          60
            if self.can_merge(&ex, &newex) {
                self.merge_extent(&search_path, &mut ex, &newex)?;

                if at_root {
                    // we are at root
                    *inode_ref.inode.root_extent_mut_at(node.position) = ex;
                }
                return Ok(());
            }

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
                if let Some(next_extent) = self.get_extent_from_node(&node, pos + 1) {
                    if self.can_merge(&next_extent, &newex) {
                        self.merge_extent(&search_path, newex, &next_extent)?;
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
                if let Some(mut prev_extent) = self.get_extent_from_node(&node, pos - 1) {
                    if self.can_merge(&prev_extent, &newex) {
                        self.merge_extent(&search_path, &mut prev_extent, &newex)?;
                        return Ok(());
                    }
                }
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
            self.insert_new_extent(inode_ref, &mut search_path, newex)?;
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

        true
    }

    fn merge_extent(
        &self,
        search_path: &SearchPath,
        left_ext: &mut Ext4Extent,
        right_ext: &Ext4Extent,
    ) -> Result<()> {
        let depth = search_path.depth as usize;
        left_ext.block_count += right_ext.block_count;

        if left_ext.first_block >= 0x20000{
            let node = &search_path.path[depth];
            let block = node.pblock_of_node;
            let new_ex_offset = core::mem::size_of::<Ext4ExtentHeader>() + core::mem::size_of::<Ext4Extent>() * (node.position);
            let mut ext4block = Block::load(self.block_device.clone(), block * BLOCK_SIZE);
            let left_ext:&mut Ext4Extent = ext4block.read_offset_as_mut(new_ex_offset);
            left_ext.block_count += 1;
            ext4block.sync_blk_to_disk(self.block_device.clone());
        }


        Ok(())
    }

    fn insert_new_extent(
        &self,
        inode_ref: &mut Ext4InodeRef,
        search_path: &mut SearchPath,
        new_extent: &mut Ext4Extent,
    ) -> Result<()> {
        let depth = search_path.depth as usize;
        let node = &mut search_path.path[depth]; // Get the node at the current depth
        let header = node.header.clone();

        // insert at root
        if depth == 0 {
            // Node is empty (no extents)
            if header.entries_count == 0 {
                *inode_ref.inode.root_extent_mut_at(node.position) = *new_extent;
                (*inode_ref.inode.root_extent_header_mut()).entries_count += 1;

                self.write_back_inode(inode_ref);
                return Ok(());
            }
            // Not empty, insert at search result pos + 1
            log::trace!("insert newex at pos {:x?} current entry_count {:x?} ex {:x?}", node.position + 1 , header.entries_count, new_extent);
            *inode_ref.inode.root_extent_mut_at(node.position + 1) = *new_extent;
            (*inode_ref.inode.root_extent_header_mut()).entries_count += 1;
            return Ok(());
        }else{
            // insert at nonroot
            log::trace!("insert newex at pos {:x?} current entry_count {:x?} ex {:x?}", node.position + 1 , header.entries_count, new_extent);

            // load block
            let node_block = node.pblock_of_node;
            let mut ext4block =
            Block::load(self.block_device.clone(), node_block as usize * BLOCK_SIZE);
            let new_ex_offset = core::mem::size_of::<Ext4ExtentHeader>() + core::mem::size_of::<Ext4Extent>() * (node.position + 1);

            // insert new extent
            let ex: &mut Ext4Extent = ext4block.read_offset_as_mut(new_ex_offset);
            *ex = *new_extent;
            let header: &mut Ext4ExtentHeader = ext4block.read_offset_as_mut(0);

            // update entry count 
            header.entries_count += 1;

            // sync to disk
            ext4block.sync_blk_to_disk(self.block_device.clone());

            return Ok(());
        }

        return_errno_with_message!(Errno::ENOTSUP, "Not supported insert extent at nonroot");
    }

    // finds empty index and adds new leaf. if no free index is found, then it requests in-depth growing.
    fn create_new_leaf(
        &self,
        inode_ref: &mut Ext4InodeRef,
        search_path: &mut SearchPath,
        new_extent: &mut Ext4Extent,
    ) -> Result<()> {
        // log::info!("search path {:x?}", search_path);

        // tree is full, time to grow in depth
        self.ext_grow_indepth(inode_ref);

        // insert again
        self.insert_extent(inode_ref, new_extent)

    }

    
    // allocates new block
    // moves top-level data (index block or leaf) into the new block
    // initializes new top-level, creating index that points to the
    // just created block
    fn ext_grow_indepth(&self, inode_ref: &mut Ext4InodeRef) -> Result<()>{
        // Try to prepend new index to old one
        let new_block = self.balloc_alloc_block(inode_ref, None)?;

        // load new block
        let mut new_ext4block =
            Block::load(self.block_device.clone(), new_block as usize * BLOCK_SIZE);

        // move top-level index/leaf into new block
        let data_to_copy = &inode_ref.inode.block;
        let data_to_copy = data_to_copy.as_ptr() as *const u8;
        unsafe{core::ptr::copy_nonoverlapping(data_to_copy, new_ext4block.data.as_mut_ptr(), 60)};
        
        // zero out unused area in the extent block
        new_ext4block.data[60..].fill(0);

        // set new block header
        let mut new_header = Ext4ExtentHeader::load_from_u8_mut(&mut new_ext4block.data);
        new_header.set_magic();
        let space = (BLOCK_SIZE - core::mem::size_of::<Ext4ExtentHeader>()) / core::mem::size_of::<Ext4Extent>();
        new_header.set_max_entries_count(space as u16);
        
        // Update top-level index: num,max,pointer
        let mut root_header = inode_ref.inode.root_extent_header_mut();
        root_header.set_entries_count(1);
        root_header.add_depth();

        let root_depth = root_header.depth;
        let root_first_extent_block = inode_ref.inode.root_extent_at(0).first_block;
        let mut root_first_index = inode_ref.inode.root_first_index_mut();
        root_first_index.store_pblock(new_block);
        if root_depth == 0 {
            // Root extent block becomes index block
            root_first_index.first_block = root_first_extent_block;
        }


        new_ext4block.sync_blk_to_disk(self.block_device.clone());
        self.write_back_inode(inode_ref);


        Ok(())
    }
    
}

impl Ext4 {
    // Assuming init state
    // depth 0 (root node)
    // +--------+--------+--------+
    // |  idx1  |  idx2  |  idx3  |
    // +--------+--------+--------+
    //     |         |         |
    //     v         v         v
    //
    // depth 1 (internal node)
    // +--------+...+--------+  +--------+...+--------+ ......
    // |  idx1  |...|  idxn  |  |  idx1  |...|  idxn  | ......
    // +--------+...+--------+  +--------+...+--------+ ......
    //     |           |         |             |
    //     v           v         v             v
    //
    // depth 2 (leaf nodes)
    // +--------+...+--------+  +--------+...+--------+  ......
    // | ext1   |...| extn   |  | ext1   |...| extn   |  ......
    // +--------+...+--------+  +--------+...+--------+  ......
    pub fn extent_remove_space(
        &self,
        inode_ref: &mut Ext4InodeRef,
        from: u32,
        to: u32,
    ) -> Result<usize> {
        // log::info!("Remove space from {:x?} to {:x?}", from, to);
        let mut search_path = self.find_extent(inode_ref, from)?;

        // for i in search_path.path.iter() {
        //     log::info!("from Path: {:x?}", i);
        // }

        let depth = search_path.depth as usize;

        /* If we do remove_space inside the range of an extent */
        let mut ex = search_path.path[depth].extent.clone().unwrap();
        if ex.get_first_block() < from
            && to < (ex.get_first_block() + ex.get_actual_len() as u32 - 1)
        {
            let mut newex = Ext4Extent::default();
            let unwritten = ex.is_unwritten();
            let ee_block = ex.first_block;
            let block_count = ex.block_count;
            let newblock = to + 1 - ee_block + ex.get_pblock() as u32;
            ex.block_count = from as u16 - ee_block as u16;

            if unwritten {
                ex.mark_unwritten();
            }
            newex.first_block = to + 1;
            newex.block_count = (ee_block + block_count as u32 - 1 - to) as u16;
            newex.start_lo = newblock as u32 & 0xffffffff;
            newex.start_hi = (((newblock as u32) << 31) << 1) as u16;

            self.insert_extent(inode_ref, &mut newex)?;

            return Ok(EOK);
        }

        // log::warn!("Remove space in depth: {:x?}", depth);

        let mut i = depth as isize;

        while i >= 0 {
            // we are at the leaf node
            // depth 0 (root node)
            // +--------+--------+--------+
            // |  idx1  |  idx2  |  idx3  |
            // +--------+--------+--------+
            //              |path
            //              v
            //              idx2
            // depth 1 (internal node)
            // +--------+--------+--------+ ......
            // |  idx1  |  idx2  |  idx3  | ......
            // +--------+--------+--------+ ......
            //              |path
            //              v
            //              ext2
            // depth 2 (leaf nodes)
            // +--------+--------+..+--------+
            // | ext1   | ext2   |..|last_ext|
            // +--------+--------+..+--------+
            //            ^            ^
            //            |            |
            //            from         to(exceed last ext, rest of the extents will be removed)
            if i as usize == depth {
                let node_pblock = search_path.path[i as usize].pblock_of_node;

                let header = search_path.path[i as usize].header.clone();
                let entries_count = header.entries_count;

                // we are at root
                if node_pblock == 0 {
                    let first_ex = inode_ref.inode.root_extent_at(0);
                    let last_ex = inode_ref.inode.root_extent_at(entries_count as usize - 1);

                    let mut leaf_from = first_ex.first_block;
                    let mut leaf_to = last_ex.first_block + last_ex.get_actual_len() as u32 - 1;
                    if leaf_from < from {
                        leaf_from = from;
                    }
                    if leaf_to > to {
                        leaf_to = to;
                    }
                    // log::trace!("from {:x?} to {:x?} leaf_from {:x?} leaf_to {:x?}", from, to, leaf_from, leaf_to);
                    self.ext_remove_leaf(inode_ref, &mut search_path, leaf_from, leaf_to)?;

                    i -= 1;
                    continue;
                }
                let ext4block =
                    Block::load(self.block_device.clone(), node_pblock as usize * BLOCK_SIZE);

                let header = search_path.path[i as usize].header.clone();
                let entries_count = header.entries_count;

                let first_ex: Ext4Extent = ext4block.read_offset_as(size_of::<Ext4ExtentHeader>());
                let last_ex: Ext4Extent = ext4block.read_offset_as(
                    size_of::<Ext4ExtentHeader>()
                        + size_of::<Ext4Extent>() * (entries_count - 1) as usize,
                );

                let mut leaf_from = first_ex.first_block;
                let mut leaf_to = last_ex.first_block + last_ex.get_actual_len() as u32 - 1;

                if leaf_from < from {
                    leaf_from = from;
                }
                if leaf_to > to {
                    leaf_to = to;
                }
                // log::trace!(
                //     "from {:x?} to {:x?} leaf_from {:x?} leaf_to {:x?}",
                //     from,
                //     to,
                //     leaf_from,
                //     leaf_to
                // );

                self.ext_remove_leaf(inode_ref, &mut search_path, leaf_from, leaf_to)?;

                i -= 1;
                continue;
            }

            // log::trace!("---at level---{:?}\n", i);

            // we are at index
            // example i=1, depth=2
            // depth 0 (root node) - 处理的索引节点
            // +--------+--------+--------+
            // |  idx1  |  idx2  |  idx3  |
            // +--------+--------+--------+
            //            |path     | 下一个要处理的节点(more_to_rm?)
            //            v         v
            //           idx2
            //
            // depth 1 (internal node)
            // +--------++--------+...+--------+
            // |  idx1  ||  idx2  |...|  idxn  |
            // +--------++--------+...+--------+
            //            |path
            //            v
            //            ext2
            // depth 2 (leaf nodes)
            // +--------+--------+..+--------+
            // | ext1   | ext2   |..|last_ext|
            // +--------+--------+..+--------+
            let header = search_path.path[i as usize].header.clone();
            if self.more_to_rm(&search_path.path[i as usize], to) {
                // todo
                // load next idx

                // go to this node's child
                i += 1;
            } else {
                if i > 0 {
                    // empty
                    if header.entries_count == 0 {
                        self.ext_remove_idx(inode_ref, &mut search_path, i as u16 - 1)?;
                    }
                }

                let idx = i as isize;
                if idx - 1 < 0 {
                    break;
                }
                i -= 1;
            }
        }

        Ok(EOK)
    }

    pub fn ext_remove_leaf(
        &self,
        inode_ref: &mut Ext4InodeRef,
        path: &mut SearchPath,
        from: u32,
        to: u32,
    ) -> Result<usize> {
        // log::trace!("Remove leaf from {:x?} to {:x?}", from, to);

        // depth 0 (root node)
        // +--------+--------+--------+
        // |  idx1  |  idx2  |  idx3  |
        // +--------+--------+--------+
        //     |         |         |
        //     v         v         v
        //     ^
        //     Current position
        let depth = inode_ref.inode.root_header_depth();
        let mut header = path.path[depth as usize].header;

        let mut new_entry_count = header.entries_count;
        let mut ex2 = Ext4Extent::default();

        /* find where to start removing */
        let pos = path.path[depth as usize].position;
        let entry_count = header.entries_count;

        // depth 1 (internal node)
        // +--------+...+--------+  +--------+...+--------+ ......
        // |  idx1  |...|  idxn  |  |  idx1  |...|  idxn  | ......
        // +--------+...+--------+  +--------+...+--------+ ......
        //     |           |         |             |
        //     v           v         v             v
        //     ^
        //     Current loaded node

        // load node data
        let node_disk_pos = path.path[depth as usize].pblock_of_node * BLOCK_SIZE;

        let mut ext4block = if node_disk_pos == 0 {
            // we are at root
            Block::load_inode_root_block(&inode_ref.inode.block)
        } else {
            Block::load(self.block_device.clone(), node_disk_pos)
        };

        // depth 2 (leaf nodes)
        // +--------+...+--------+  +--------+...+--------+  ......
        // | ext1   |...| extn   |  | ext1   |...| extn   |  ......
        // +--------+...+--------+  +--------+...+--------+  ......
        //     ^
        //     Current start extent

        // start from pos
        for i in pos..entry_count as usize {
            let ex: &mut Ext4Extent = ext4block
                .read_offset_as_mut(size_of::<Ext4ExtentHeader>() + i * size_of::<Ext4Extent>());

            if ex.first_block > to {
                break;
            }

            let mut new_len = 0;
            let mut start = ex.first_block;
            let mut new_start = ex.first_block;

            let mut len = ex.get_actual_len();
            let mut newblock = ex.get_pblock();

            // Initial state:
            // +--------+...+--------+  +--------+...+--------+  ......
            // | ext1   |...| ext2   |  | ext3   |...| extn   |  ......
            // +--------+...+--------+  +--------+...+--------+  ......
            //               ^                    ^
            //              from                  to

            // Case 1: Remove a portion within the extent
            if start < from {
                len -= from as u16 - start as u16;
                new_len = from - start;
                start = from;
            } else {
                // Case 2: Adjust extent that partially overlaps the 'to' boundary
                if start + len as u32 - 1 > to {
                    new_len = start + len as u32 - 1 - to;
                    len -= new_len as u16;
                    new_start = to + 1;
                    newblock += (to + 1 - start) as u64;
                    ex2 = *ex;
                }
            }

            // After removing range from `from` to `to`:
            // +--------+...+--------+  +--------+...+--------+  ......
            // | ext1   |...[removed]|  |[removed]|...| extn   |  ......
            // +--------+...+--------+  +--------+...+--------+  ......
            //               ^                    ^
            //              from                  to
            //                                  new_start

            // Remove blocks within the extent
            self.ext_remove_blocks(inode_ref, ex, start, start + len as u32 - 1);

            ex.first_block = new_start;
            // log::trace!("after remove leaf ex first_block {:x?}", ex.first_block);

            if new_len == 0 {
                new_entry_count -= 1;
            } else {
                let unwritten = ex.is_unwritten();
                ex.store_pblock(newblock as u64);
                ex.block_count = new_len as u16;

                if unwritten {
                    ex.mark_unwritten();
                }
            }
        }

        // Move remaining extents to the start:
        // Before:
        // +--------+--------+...+--------+
        // | ext3   | ext4   |...| extn   |
        // +--------+--------+...+--------+
        //      ^       ^
        //      rm      rm
        // After:
        // +--------+.+--------+--------+...
        // | ext1   |.| extn   | [empty]|...
        // +--------+.+--------+--------+...

        // Move any remaining extents to the starting position of the node.
        if ex2.first_block > 0 {
            let start_index = size_of::<Ext4ExtentHeader>() + pos * size_of::<Ext4Extent>();
            let end_index =
                size_of::<Ext4ExtentHeader>() + entry_count as usize * size_of::<Ext4Extent>();
            let remaining_extents: Vec<u8> = ext4block.data[start_index..end_index].to_vec();
            ext4block.data[size_of::<Ext4ExtentHeader>()
                ..size_of::<Ext4ExtentHeader>() + remaining_extents.len()]
                .copy_from_slice(&remaining_extents);
        }

        // Update the entries count in the header
        header.entries_count = new_entry_count;

        /*
         * If the extent pointer is pointed to the first extent of the node, and
         * there's still extents presenting, we may need to correct the indexes
         * of the paths.
         */
        if pos == 0 && new_entry_count > 0 {
            self.ext_correct_indexes(path)?;
        }

        /* if this leaf is free, then we should
         * remove it from index block above */
        if new_entry_count == 0 {
            // if we are at root?
            if path.path[depth as usize].pblock_of_node == 0 {
                return Ok(EOK);
            }
            self.ext_remove_idx(inode_ref, path, depth - 1)?;
        } else if depth > 0 {
            // go to next index
            path.path[depth as usize - 1].position += 1;
        }

        Ok(EOK)
    }

    fn ext_remove_index_block(&self, inode_ref: &mut Ext4InodeRef, index: &mut Ext4ExtentIndex) {
        let block_to_free = index.get_pblock();

        // log::trace!("remove index's block {:x?}", block_to_free);
        self.balloc_free_blocks(inode_ref, block_to_free as _, 1);
    }

    fn ext_remove_idx(
        &self,
        inode_ref: &mut Ext4InodeRef,
        path: &mut SearchPath,
        depth: u16,
    ) -> Result<usize> {
        // log::trace!("Remove index at depth {:x?}", depth);

        // Initial state:
        // +--------+--------+--------+
        // |  idx1  |  idx2  |  idx3  |
        // +--------+--------+--------+
        //           ^
        // Current index to remove (pos=1)

        // Removing index:
        // +--------+--------+--------+
        // |  idx1  |[empty] |  idx3  |
        // +--------+--------+--------+
        //           ^
        // Current index to remove

        // After moving remaining indexes:
        // +--------+--------+--------+
        // |  idx1  |  idx3  |[empty] |
        // +--------+--------+--------+
        //           ^
        // Current index to remove

        let mut i = depth as usize;
        let mut header = path.path[i].header;

        // 获取要删除的索引块
        let leaf_block = path.path[i].index.unwrap().get_pblock();

        // 如果当前索引不是最后一个索引，将后续的索引前移
        if path.path[i].position != header.entries_count as usize - 1 {
            let start_pos = size_of::<Ext4ExtentHeader>()
                + path.path[i].position * size_of::<Ext4ExtentIndex>();
            let end_pos = size_of::<Ext4ExtentHeader>()
                + (header.entries_count as usize) * size_of::<Ext4ExtentIndex>();

            let node_disk_pos = path.path[i].pblock_of_node as usize * BLOCK_SIZE;
            let mut ext4block = Block::load(self.block_device.clone(), node_disk_pos);

            let remaining_indexes: Vec<u8> =
                ext4block.data[start_pos + size_of::<Ext4ExtentIndex>()..end_pos].to_vec();
            ext4block.data[start_pos..start_pos + remaining_indexes.len()]
                .copy_from_slice(&remaining_indexes);
            let remaining_size = remaining_indexes.len();

            // 清空剩余位置
            let empty_start = start_pos + remaining_size;
            let empty_end = end_pos;
            ext4block.data[empty_start..empty_end].fill(0);
        }

        // 更新头部的entries_count
        header.entries_count -= 1;

        // 释放索引块
        self.ext_remove_index_block(inode_ref, &mut path.path[i].index.unwrap());

        // Updating parent index if necessary:
        // +--------+--------+--------+
        // |  idx1  |  idx3  |[empty] |
        // +--------+--------+--------+
        //           ^
        // Updated parent index if necessary

        // 如果当前层不是根，需要检查是否需要更新父节点索引
        while i > 0 {
            if path.path[i].position != 0 {
                break;
            }

            let parent_idx = i - 1;
            let parent_index = &mut path.path[parent_idx].index.unwrap();
            let current_index = &path.path[i].index.unwrap();

            parent_index.first_block = current_index.first_block;
            self.write_back_inode(inode_ref);

            i -= 1;
        }

        Ok(EOK)
    }

    /// Correct the first block of the parent index.
    fn ext_correct_indexes(&self, path: &mut SearchPath) -> Result<usize> {
        // if child get removed from parent, we need to update the parent's first_block
        let mut depth = path.depth as usize;

        // depth 2:
        // +--------+--------+--------+
        // |[empty] |  ext2  |  ext3  |
        // +--------+--------+--------+
        // ^
        // pos=0, ext1_first_block=0(removed) parent index first block=0

        // depth 2:
        // +--------+--------+--------+
        // |  ext2  |  ext3  |[empty] |
        // +--------+--------+--------+
        // ^
        // pos=0, now first_block=ext2_first_block

        // 更新父节点索引：
        // depth 1:
        // +-----------------------+
        // | idx1_2 |...| idx1_n   |
        // +-----------------------+
        //     ^
        //     更新父节点索引(first_block)

        // depth 0:
        // +--------+--------+--------+
        // |  idx1  |  idx2  |  idx3  |
        // +--------+--------+--------+
        //     |
        //     更新根节点索引(first_block)

        while depth > 0 {
            let parent_idx = depth - 1;

            // 获取当前层的 extent
            if let Some(child_extent) = path.path[depth].extent {
                // 获取父节点
                let parent_node = &mut path.path[parent_idx];
                // 获取父节点的索引，并更新 first_block
                if let Some(ref mut parent_index) = parent_node.index {
                    parent_index.first_block = child_extent.first_block;
                }
            }

            depth -= 1;
        }

        Ok(EOK)
    }

    fn ext_remove_blocks(
        &self,
        inode_ref: &mut Ext4InodeRef,
        ex: &mut Ext4Extent,
        from: u32,
        to: u32,
    ) {
        let len = to - from + 1;
        let num = from - ex.first_block;
        let start: u32 = ex.get_pblock() as u32 + num;
        self.balloc_free_blocks(inode_ref, start as _, len);
    }

    pub fn more_to_rm(&self, path: &ExtentPathNode, to: u32) -> bool {
        let header = path.header.clone();

        // No Sibling exists
        if header.entries_count == 1 {
            return false;
        }

        let pos = path.position;
        if pos > header.entries_count as usize - 1 {
            return false;
        }

        // Check if index is out of bounds
        if let Some(index) = path.index {
            let last_index_pos = header.entries_count as usize - 1;
            let node_disk_pos = path.pblock_of_node * BLOCK_SIZE;
            let ext4block = Block::load(self.block_device.clone(), node_disk_pos);
            let last_index: Ext4ExtentIndex =
                ext4block.read_offset_as(size_of::<Ext4ExtentIndex>() * last_index_pos);

            if path.position > last_index_pos || index.first_block > last_index.first_block {
                return false;
            }

            // Check if index's first_block is greater than 'to'
            if index.first_block > to {
                return false;
            }
        }

        true
    }
}
