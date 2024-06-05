use crate::prelude::*;
use crate::return_errno_with_message;

use super::*;

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

/// Extent tree node. Includes the header, the data.
#[derive(Clone, Debug)]
pub struct ExtentNode {
    pub header: Ext4ExtentHeader,
    pub data: NodeData,
    pub is_root: bool,
}

/// Data of extent tree.
#[derive(Clone, Debug)]
pub enum NodeData {
    Root([u32; 15]),
    Internal(Vec<u8>), // size = BLOCK_SIZE
}

/// Search path in the extent tree.
#[derive(Clone, Debug)]
pub struct SearchPath {
    pub depth: u16,                      // current depth
    pub maxdepth: u16,                   // max depth
    pub path: Vec<ExtentPathNode>,       // search result of each level
}

/// Extent tree node search result
#[derive(Clone, Debug)]
pub struct ExtentPathNode {
    pub header: Ext4ExtentHeader,        // save header for convenience
    pub index: Option<Ext4ExtentIndex>,  // for convenience(you can get index through pos of extent node)
    pub extent: Option<Ext4Extent>,      // same reason as above
    pub position: usize,                 // position of search result in the node
    pub pblock: u64,                     // disk position of this node
}

/// load methods for Ext4ExtentHeader
impl Ext4ExtentHeader {
    /// Load the extent header from u32 array.
    pub fn load_from_u32(data: &[u32]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u32 array mutably.
    pub fn load_from_u32_mut(data: &mut [u32]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }

    /// Load the extent header from u8 array.
    pub fn load_from_u8(data: &[u8]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u8 array mutably.
    pub fn load_from_u8_mut(data: &mut [u8]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }

    /// Is the node a leaf node?
    pub fn is_leaf(&self) -> bool {
        self.depth == 0
    }
}

/// load methods for Ext4ExtentIndex
impl Ext4ExtentIndex {
    /// Load the extent header from u32 array.
    pub fn load_from_u32(data: &[u32]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u32 array mutably.
    pub fn load_from_u32_mut(data: &mut [u32]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }

    /// Load the extent header from u8 array.
    pub fn load_from_u8(data: &[u8]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u8 array mutably.
    pub fn load_from_u8_mut(data: &mut [u8]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }
}

/// load methods for Ext4Extent
impl Ext4Extent {
    /// Load the extent header from u32 array.
    pub fn load_from_u32(data: &[u32]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u32 array mutably.
    pub fn load_from_u32_mut(data: &mut [u32]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }

    /// Load the extent header from u8 array.
    pub fn load_from_u8(data: &[u8]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u8 array mutably.
    pub fn load_from_u8_mut(data: &mut [u8]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }
}

impl ExtentNode {
    /// Load the extent node from the data.
    pub fn load_from_data(data: &[u8], is_root: bool) -> Result<Self> {
        if is_root {
            if data.len() != 15 * 4 {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }

            let mut root_data = [0u32; 15];
            for (i, chunk) in data.chunks(4).enumerate() {
                root_data[i] = u32::from_le_bytes(chunk.try_into().unwrap());
            }

            let header = Ext4ExtentHeader::load_from_u32(&root_data);

            Ok(ExtentNode {
                header,
                data: NodeData::Root(root_data),
                is_root,
            })
        } else {
            if data.len() != BLOCK_SIZE {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }
            let header = Ext4ExtentHeader::load_from_u8(&data[..size_of::<Ext4ExtentHeader>()]);
            Ok(ExtentNode {
                header,
                data: NodeData::Internal(data.to_vec()),
                is_root,
            })
        }
    }

    /// Load the extent node from the data mutably.
    pub fn load_from_data_mut(data: &mut [u8], is_root: bool) -> Result<Self> {
        if is_root {
            if data.len() != 15 * 4 {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }

            let mut root_data = [0u32; 15];
            for (i, chunk) in data.chunks(4).enumerate() {
                root_data[i] = u32::from_le_bytes(chunk.try_into().unwrap());
            }

            let header = Ext4ExtentHeader::load_from_u32_mut(&mut root_data);

            Ok(ExtentNode {
                header,
                data: NodeData::Root(root_data),
                is_root,
            })
        } else {
            if data.len() != BLOCK_SIZE {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }
            let header =
                Ext4ExtentHeader::load_from_u8_mut(&mut data[..size_of::<Ext4ExtentHeader>()]);
            Ok(ExtentNode {
                header,
                data: NodeData::Internal(data.to_vec()),
                is_root,
            })
        }
    }
}

impl ExtentNode {
    /// Binary search for the extent that contains the given block.
    pub fn binsearch_extent(&self, lblock: Ext4Lblk) -> Option<(Ext4Extent, usize)> {

        // empty node
        if self.header.entries_count == 0 {
            match &self.data {
                NodeData::Root(root_data) => {
                    let extent = Ext4Extent::load_from_u32(&root_data[3..]);
                    return Some((extent, 0));
                }
                NodeData::Internal(internal_data) => {
                    let extent = Ext4Extent::load_from_u8(&internal_data[12..]);
                    return Some((extent, 0));
                }
            }
        }

        match &self.data {
            NodeData::Root(root_data) => {
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let extents = &root_data[start..];

                let mut l = 0;
                let mut r = (self.header.entries_count - 1) as usize;

                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = m * size_of::<Ext4Extent>() / 4;
                    let extent = Ext4Extent::load_from_u32(&extents[offset..]);

                    if lblock < extent.first_block {
                        if m == 0 {
                            break;
                        }
                        r = m - 1;
                    } else if lblock >= extent.first_block + extent.block_count as Ext4Lblk {
                        l = m + 1;
                    } else {
                        return Some((extent, m));
                    }
                }
                None
            }
            NodeData::Internal(internal_data) => {
                let start = size_of::<Ext4ExtentHeader>();
                let extents = &internal_data[start..];

                let mut l = 0;
                let mut r = (self.header.entries_count - 1) as usize;

                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = m * size_of::<Ext4Extent>();
                    let extent = Ext4Extent::load_from_u8(&extents[offset..]);

                    if lblock < extent.first_block {
                        if m == 0 {
                            break;
                        }
                        r = m - 1;
                    } else if lblock >= extent.first_block + extent.block_count as Ext4Lblk {
                        l = m + 1;
                    } else {
                        return Some((extent, m));
                    }
                }
                None
            }
        }
    }

    /// Binary search for the closest index of the given block.
    pub fn binsearch_idx(&self, lblock: Ext4Lblk) -> Option<usize> {

        if self.header.entries_count == 0 {
            return None;
        }

        match &self.data {
            NodeData::Root(root_data) => {
                // Root node handling
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let indexes = &root_data[start..];

                let mut l = 1; // Skip the first index
                let mut r = self.header.entries_count as usize - 1;

                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = m * size_of::<Ext4ExtentIndex>() / 4; // Convert to u32 offset
                    let extent_index = Ext4ExtentIndex::load_from_u32(&indexes[offset..]);

                    if lblock < extent_index.first_block {
                        if m == 0 {
                            break; // Prevent underflow
                        }
                        r = m - 1;
                    } else {
                        l = m + 1;
                    }
                }

                if l == 0 {
                    return None;
                }

                Some(l - 1)
            }
            NodeData::Internal(internal_data) => {
                // Internal node handling
                let start = size_of::<Ext4ExtentHeader>();
                let indexes = &internal_data[start..];

                let mut l = 0;
                let mut r = (self.header.entries_count - 1) as usize;

                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = m * size_of::<Ext4ExtentIndex>();
                    let extent_index = Ext4ExtentIndex::load_from_u8(&indexes[offset..]);

                    if lblock < extent_index.first_block {
                        if m == 0 {
                            break; // Prevent underflow
                        }
                        r = m - 1;
                    } else {
                        l = m + 1;
                    }
                }

                if l == 0 {
                    return None;
                }

                Some(l - 1)
            }
        }
    }

    /// Get the index node at the given position.
    pub fn get_index(&self, pos: usize) -> Result<Ext4ExtentIndex> {
        match &self.data {
            NodeData::Root(root_data) => {
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let indexes = &root_data[start..];
                let offset = pos * size_of::<Ext4ExtentIndex>() / 4;
                Ok(Ext4ExtentIndex::load_from_u32(&indexes[offset..]))
            }
            NodeData::Internal(internal_data) => {
                let start = size_of::<Ext4ExtentHeader>();
                let indexes = &internal_data[start..];
                let offset = pos * size_of::<Ext4ExtentIndex>();
                Ok(Ext4ExtentIndex::load_from_u8(&indexes[offset..]))
            }
        }
    }

    /// Get the extent node at the given position.
    pub fn get_extent(&self, pos: usize) -> Option<Ext4Extent> {
        match &self.data {
            NodeData::Root(root_data) => {
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let extents = &root_data[start..];
                let offset = pos * size_of::<Ext4Extent>() / 4;
                Some(Ext4Extent::load_from_u32(&extents[offset..]))
            }
            NodeData::Internal(internal_data) => {
                let start = size_of::<Ext4ExtentHeader>();
                let extents = &internal_data[start..];
                let offset = pos * size_of::<Ext4Extent>();
                Some(Ext4Extent::load_from_u8(&extents[offset..]))
            }
        }
    }
}

impl Ext4Extent {
    /// Get the first block number(logical) of the extent.
    pub fn get_first_block(&self) -> u32 {
        self.first_block
    }

    /// Set the first block number(logical) of the extent.
    pub fn set_first_block(&mut self, first_block: u32) {
        self.first_block = first_block;
    }

    /// Get the starting physical block number of the extent.
    pub fn get_pblock(&self) -> u64 {
        let lo = u64::from(self.start_lo);
        let hi = u64::from(self.start_hi) << 32;
        lo | hi
    }

    /// Stores the physical block number to which this extent points.
    pub fn store_pblock(&mut self, pblock: u64) {
        self.start_lo = pblock as u32 & 0xffffffff;
        self.start_hi = (((pblock as u32) << 31) << 1) as u16;
    }

    /// Returns true if the extent is unwritten.
    pub fn is_unwritten(&self) -> bool {
        self.block_count > EXT_INIT_MAX_LEN
    }

    /// Returns the actual length of the extent.
    pub fn get_actual_len(&self) -> u16 {
        if self.is_unwritten() {
            self.block_count - EXT_INIT_MAX_LEN
        } else {
            self.block_count
        }
    }

    /// Can merge next extent to this extent?
    pub fn can_append(&self, next: &Self) -> bool {
        self.first_block + self.get_actual_len() as u32 == next.first_block
            && if self.is_unwritten() {
                self.get_actual_len() + next.get_actual_len() <= EXT_UNWRITTEN_MAX_LEN
            } else {
                self.get_actual_len() + next.get_actual_len() <= EXT_INIT_MAX_LEN
            }
    }

    /// Can merge this extent to previous extent?
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

impl SearchPath {
    pub fn new() -> Self {
        SearchPath {
            depth: 0,
            maxdepth: 4,
            path: vec![],
        }
    }
}
