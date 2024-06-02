// use super::*;
use crate::consts::*;
use crate::prelude::*;
use core::mem::size_of;
use crate::BlockDevice;

/// Structure representing the header of an Ext4 extent.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentHeader {
    /// Magic number, 0xF30A.
    pub magic: u16,

    /// Number of valid entries following the header.
    pub entries_count: u16,

    /// Maximum number of entries that could follow the header.
    pub max_entries_count: u16,

    /// Depth of this extent node in the extent tree. Depth 0 indicates that this node points to data blocks.
    pub depth: u16,

    /// Generation of the tree (used by Lustre, but not standard in ext4).
    pub generation: u32,
}

/// Structure representing an index node within an extent tree.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentIndex {
    /// Block number from which this index node starts.
    pub first_block: u32,

    /// Lower 32-bits of the block number to which this index points.
    pub leaf_lo: u32,

    /// Upper 16-bits of the block number to which this index points.
    pub leaf_hi: u16,

    /// Padding for alignment.
    pub padding: u16,
}

/// Structure representing an Ext4 extent.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4Extent {
    /// First file block number that this extent covers.
    pub first_block: u32,

    /// Number of blocks covered by this extent.
    pub block_count: u16,

    /// Upper 16-bits of the block number to which this extent points.
    pub start_hi: u16,

    /// Lower 32-bits of the block number to which this extent points.
    pub start_lo: u32,
}

/// Extent Path handling for navigating through extent trees.
#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentPath {
    // Physical block number
    pub p_block: u64,
    // Single block descriptor
    // pub block: Ext4Block,
    // Depth of this extent node
    pub depth: u16,
    // Max depth of the extent tree
    pub maxdepth: u16,
    // Pointer to the extent header
    pub header: *mut Ext4ExtentHeader,
    // Pointer to the index in the current node
    pub index: *mut Ext4ExtentIndex,
    // Pointer to the extent in the current node
    pub extent: *mut Ext4Extent,
}

impl Default for Ext4ExtentPath {
    fn default() -> Self {
        Self {
            p_block: 0,
            // block: Ext4Block::default(),
            depth: 0,
            maxdepth: 0,
            header: core::ptr::null_mut(),
            index: core::ptr::null_mut(),
            extent: core::ptr::null_mut(),
        }
    }
}

impl<T> TryFrom<&[T]> for Ext4ExtentHeader {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = data;
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl<T> TryFrom<&[T]> for Ext4ExtentIndex {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = &data[..size_of::<Ext4ExtentIndex>()];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl<T> TryFrom<&[T]> for Ext4Extent {
    type Error = u64;
    fn try_from(data: &[T]) -> core::result::Result<Self, u64> {
        let data = &data[..];
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const _) })
    }
}

impl Ext4ExtentHeader{
    pub fn try_from_u32(data: &mut [u32]) -> Self{
        // let data = data;
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }
}


impl Ext4ExtentIndex {
    /// Returns the physical block number represented by this index.
    pub fn pblock(&self) -> u64 {
        // Get the lower 32 bits of the block number
        let pblock_lo = self.leaf_lo as u64;

        // Get the upper 16 bits and shift them into the high part of the result
        let pblock_hi = self.leaf_hi as u64;
        let pblock = pblock_lo | (pblock_hi << 32);

        pblock
    }
}


impl Ext4ExtentHeader {
    pub fn new(magic: u16, entries: u16, max_entries: u16, depth: u16, generation: u32) -> Self {
        Self {
            magic,
            entries_count: entries,
            max_entries_count: max_entries,
            depth,
            generation,
        }
    }

    pub fn set_depth(&mut self, depth: u16) {
        self.depth = depth;
    }

    pub fn set_entries_count(&mut self, entries_count: u16) {
        self.entries_count = entries_count;
    }

    pub fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    pub fn set_magic(&mut self) {
        self.magic = EXT4_EXTENT_MAGIC;
    }

    pub fn set_max_entries_count(&mut self, max_entries_count: u16) {
        self.max_entries_count = max_entries_count;
    }
}

impl Ext4Extent {
    pub fn is_unwritten(&self) -> bool {
        self.block_count > EXT_INIT_MAX_LEN
    }

    pub fn get_actual_len(&self) -> u16 {
        if self.is_unwritten() {
            self.block_count - EXT_INIT_MAX_LEN
        } else {
            self.block_count
        }
    }

    pub fn pblock(&self) -> u32 {
        ((self.start_lo as u64) | (self.start_hi as u64) << 32) as u32
    }

    pub fn can_append(&self, next: &Self) -> bool {
        self.first_block + self.get_actual_len() as u32 == next.first_block
            && if self.is_unwritten() {
                self.get_actual_len() + next.get_actual_len() <= EXT_UNWRITTEN_MAX_LEN
            } else {
                self.get_actual_len() + next.get_actual_len() <= EXT_INIT_MAX_LEN
            }
    }

    pub fn can_prepend(&self, prev: &Self) -> bool {
        prev.first_block + prev.get_actual_len() as u32 == self.first_block
            && if self.is_unwritten() {
                self.get_actual_len() + prev.get_actual_len() <= EXT_UNWRITTEN_MAX_LEN
            } else {
                self.get_actual_len() + prev.get_actual_len() <= EXT_INIT_MAX_LEN
            }
    }
    /// Marks the extent as unwritten.
    pub fn mark_unwritten(&mut self) {
        self.block_count |= EXT_INIT_MAX_LEN;
    }

    pub fn store_pblock(&mut self, pblock: u64) {
        self.start_lo = pblock as u32 & 0xffffffff;
        self.start_hi = (((pblock as u32) << 31) << 1) as u16;
    }
}

/// Additional utility functions and trait implementations for performing operations
/// such as binary search and getters for first and last extents.
impl Ext4ExtentHeader {
    /// Get a pointer to the first extent from a given header.
    pub unsafe fn first_extent(&self) -> *const Ext4Extent {
        let offset = size_of::<Ext4ExtentHeader>();
        (self as *const Self as *const u8).add(offset) as *const Ext4Extent
    }

    /// Get a mutable pointer to the first extent from a given header.
    pub unsafe fn first_extent_mut(&mut self) -> *mut Ext4Extent {
        let offset = size_of::<Ext4ExtentHeader>();
        (self as *mut Self as *mut u8).add(offset) as *mut Ext4Extent
    }

    /// Get a pointer to the last extent from a given header.
    pub unsafe fn last_extent(&self) -> *const Ext4Extent {
        let offset = size_of::<Ext4ExtentHeader>();
        let ext_size = size_of::<Ext4Extent>();
        let last_index = self.entries_count as usize - 1;
        (self as *const Self as *const u8).add(offset + last_index * ext_size) as *const Ext4Extent
    }

    /// Get a mutable pointer to the last extent from a given header.
    pub unsafe fn last_extent_mut(&mut self) -> *mut Ext4Extent {
        let offset = size_of::<Ext4ExtentHeader>();
        let ext_size = size_of::<Ext4Extent>();
        let last_index = self.entries_count as usize - 1;
        (self as *mut Self as *mut u8).add(offset + last_index * ext_size) as *mut Ext4Extent
    }

    /// Get a pointer to the first extent index from a given header.
    pub unsafe fn first_extent_index(&self) -> *const Ext4ExtentIndex {
        let offset = size_of::<Ext4ExtentHeader>();
        (self as *const Self as *mut u8).add(offset) as *mut Ext4ExtentIndex
    }

    /// Get a mutable pointer to the first extent index from a given header.
    pub unsafe fn first_extent_index_mut(&mut self) -> *mut Ext4ExtentIndex {
        let offset = size_of::<Ext4ExtentHeader>();
        (self as *mut Self as *mut u8).add(offset) as *mut Ext4ExtentIndex
    }

    /// Get a pointer to the last extent index from a given header.
    pub unsafe fn last_extent_index(&self) -> *const Ext4ExtentIndex {
        let offset = size_of::<Ext4ExtentHeader>();
        let idx_size = size_of::<Ext4ExtentIndex>();
        let last_index = self.entries_count as usize - 1;
        (self as *const Self as *mut u8).add(offset + last_index * idx_size) as *mut Ext4ExtentIndex
    }

    /// Get a mutable pointer to the last extent index from a given header.
    pub unsafe fn last_extent_index_mut(&mut self) -> *mut Ext4ExtentIndex {
        let offset = size_of::<Ext4ExtentHeader>();
        let idx_size = size_of::<Ext4ExtentIndex>();
        let last_index = self.entries_count as usize - 1;
        (self as *mut Self as *mut u8).add(offset + last_index * idx_size) as *mut Ext4ExtentIndex
    }
}

impl Ext4ExtentPath {
    /// Searches for the specified block within the extents managed by the path.
    /// Returns `true` if the block is found within any of the extents.
    pub fn search_extent(&mut self, block: u32) -> bool {
        // Use `as_ref()` to convert the raw pointer to a reference safely
        let header = unsafe { self.header.as_ref() };

        // Proceed only if the header is valid and there are entries
        if let Some(header) = header {
            if header.entries_count == 0 {
                return false;
            }

            // Calculate the starting pointer for the extents
            let mut extent = unsafe {
                self.header.add(1) as *mut Ext4Extent // Point to the first extent
            };

            // Iterate over all extents
            for _i in 0..header.entries_count {
                let ext = unsafe { &*extent };
                // Check if the block number falls within this extent
                if block >= ext.first_block && block <= (ext.first_block + ext.block_count as u32) {
                    self.extent = extent;
                    return true;
                }

                // Move to the next extent
                extent = unsafe { extent.add(1) };
            }
        }

        false
    }

    /// Perform binary search on extent tree to find a specific block.
    pub unsafe fn binsearch_extent(&mut self, block: u32) -> bool {
        if (*self.header).entries_count == 0 {
            return false;
        }
        let header_ref = match self.header.as_mut() {
            Some(h) => h,
            None => return false, // Early return if the pointer is null
        };

        let mut left = header_ref.first_extent_mut().add(1);
        let mut right = header_ref.last_extent_mut();
        while left <= right {
            let mid = left.add((right as usize - left as usize) / 2);
            if (*mid).first_block > block {
                right = mid.sub(1);
            } else if (*mid).first_block + (*mid).block_count as u32 > block {
                left = mid.add(1);
            } else {
                self.extent = mid;
                return true;
            }
        }
        false
    }

    /// Perform binary search on indices to find a specific block.
    pub fn binsearch_extentidx(&mut self, block: u32) -> bool {
        unsafe{
            if (*self.header).entries_count == 0 {
                return false;
            }
    
            let header_ref = match self.header.as_mut() {
                Some(h) => h,
                None => return false, // Early return if the pointer is null
            };
    
            let mut left = header_ref.first_extent_index_mut().add(1);
            let mut right = header_ref.last_extent_index_mut();
            while left <= right {
                let mid =left.add((right as usize - left as usize) / 2) ;
                if (*mid).first_block > block {
                    right = mid.sub(1);
                } else if (*mid).first_block + size_of::<Ext4ExtentIndex>() as u32 > block {
                    left = mid.add(1);
                } else {
                    self.index = mid;
                    return true;
                }
            }
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ext4ExtentPathNew {
    pub depth: usize,

    /// First file block number that this extent covers.
    pub first_block: u32,

    /// Number of blocks covered by this extent.
    pub block_count: u16,

    /// Upper 16-bits of the block number to which this extent points.
    pub start_hi: u16,

    /// Lower 32-bits of the block number to which this extent points.
    pub start_lo: u32,

    pub p_block: Option<u64>,          
}


#[derive(Debug, Clone)]
pub struct ExtentTreeNode {
    pub header: Ext4ExtentHeader,
    pub extents: Vec<Ext4Extent>,
    pub indexes: Vec<Ext4ExtentIndex>,
}

impl ExtentTreeNode {
    pub fn load_from_header(data: &[u32]) -> Self {
        let extent_header = Ext4ExtentHeader::try_from(data).unwrap();
        let mut extents: Vec<Ext4Extent> = Vec::new();
        let mut indexes: Vec<Ext4ExtentIndex> = Vec::new();

        if extent_header.depth == 0 {
            for en in 0..extent_header.entries_count {
                let idx = (3 + en * 3) as usize;
                let extent = Ext4Extent::try_from(&data[idx..]).unwrap();
                extents.push(extent)
            }
        } else {
            // only have extent_index
            for en in 0..extent_header.entries_count {
                let idx = (3 + en * 3) as usize;
                let extent_idx = Ext4ExtentIndex::try_from(&data[idx..]).unwrap();
                indexes.push(extent_idx)
            }
        }

        Self {
            header: extent_header,
            extents: extents,
            indexes: indexes,
        }
    }

    pub fn load_node(&self, data: &[u8]) -> Self {
        let extent_header = Ext4ExtentHeader::try_from(data).unwrap();
        let mut extents: Vec<Ext4Extent> = Vec::new();
        let mut indexes: Vec<Ext4ExtentIndex> = Vec::new();

        if extent_header.depth == 0 {
            for en in 0..extent_header.entries_count {
                let idx = (12 + en * 12) as usize;
                let extent = Ext4Extent::try_from(&data[idx..]).unwrap();
                extents.push(extent)
            }
        } else {
            // only have extent_index
            for en in 0..extent_header.entries_count {
                let idx = (12 + en * 12) as usize;
                let extent_idx = Ext4ExtentIndex::try_from(&data[idx..]).unwrap();
                indexes.push(extent_idx)
            }
        }
        Self {
            header: extent_header,
            extents: extents,
            indexes: indexes,
        }
    }

    pub fn find_extent(
        &self,
        block_id: Ext4Lblk,
        block_device: Arc<dyn BlockDevice>,
        path: &mut Vec<Ext4ExtentPathNew>,
    ) {
        if self.header.depth == 0 {
            // 叶节点
            for extent in &self.extents {
                if block_id >= extent.first_block
                    && block_id < extent.first_block + extent.block_count as u32
                {
                    path.push(Ext4ExtentPathNew {
                        depth: self.header.depth as usize,
                        first_block: extent.first_block,
                        block_count: extent.block_count,
                        start_lo: extent.start_lo,
                        start_hi: extent.start_hi,
                        p_block: Some(extent.pblock() as u64),
                    });
                    return;
                }
            }
        } else {
            // 索引节点
            for index in &self.indexes {
                if block_id >= index.first_block {
                    let node_data = block_device.read_offset(index.leaf_lo as usize * BLOCK_SIZE);
                    let child_node = self.load_node(&node_data);

                    path.push(Ext4ExtentPathNew {
                        depth: self.header.depth as usize,
                        first_block: index.first_block,
                        block_count: index.leaf_lo as u16,
                        start_lo: index.leaf_hi as u32,
                        start_hi: index.padding,
                        p_block: Some(index.pblock() as u64),
                    });
                    return child_node.find_extent(block_id, block_device.clone(), path);
                }
            }
        }
    }    
}