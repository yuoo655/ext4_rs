use crate::prelude::*;


use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use core::any::Any;
use core::marker::Send;
use core::marker::Sync;
use core::mem::size_of;

#[cfg(feature = "blockcache")]
use spin::Mutex;

pub trait BlockDevice: Send + Sync + Any {
    fn read_offset(&self, offset: usize) -> alloc::vec::Vec<u8>;
    fn write_offset(&self, offset: usize, data: &[u8]);
}

#[cfg(feature = "blockcache")]
#[derive(Clone)]
pub struct Block {
    pub disk_offset: usize,
    pub data: Vec<u8>,
    pub cache: Arc<Mutex<LruCache>>,
}

#[cfg(not(feature = "blockcache"))]
pub struct Block {
    pub disk_offset: usize,
    pub data: Vec<u8>,
}

impl Block {
    /// Load the block from the disk.
    #[cfg(feature = "blockcache")]
    pub fn load(
        block_device: Arc<dyn BlockDevice>,
        offset: usize,
        cache: Arc<Mutex<LruCache>>,
    ) -> Self {
        // log::info!("block load offset {:x?}", offset);
        let cache_clone = Arc::clone(&cache);
        let mut cache = cache.lock();
        if let Some(block) = cache.get(offset) {
            // log::info!("lru cache hit");
            return Block {
                disk_offset: offset,
                data: block.data.clone(),
                cache: cache_clone,
            };
        }
        let data = block_device.read_offset(offset);
        let block = Block {
            disk_offset: offset,
            data,
            cache: cache_clone,
        };
        cache.put(offset, block.clone());
        block
    }

    #[cfg(not(feature = "blockcache"))]
    pub fn load(block_device: Arc<dyn BlockDevice>, offset: usize) -> Self {
        let data = block_device.read_offset(offset);
        Block {
            disk_offset: offset,
            data,
        }
    }

    /// Load the block from inode block
    pub fn load_inode_root_block(data: &[u32; 15]) -> Self {
        let data_bytes = unsafe { core::mem::transmute::<&[u32; 15], &[u8; 60]>(data) };
        #[cfg(feature = "blockcache")]
        return Block {
            disk_offset: 0,
            data: data_bytes.to_vec(),
            cache: Arc::new(Mutex::new(LruCache::new(10))),
        };
        #[cfg(not(feature = "blockcache"))]
        return Block {
            disk_offset: 0,
            data: data_bytes.to_vec(),
        };
    }

    /// Read the block as a specific type.
    pub fn read_as<T: Copy>(&self) -> T {
        unsafe {
            let ptr = self.data.as_ptr() as *const T;
            ptr.read_unaligned()
        }
    }

    /// Read the block as a specific type at a specific offset.
    pub fn read_offset_as<T: Copy>(&self, offset: usize) -> T {
        unsafe {
            let ptr = self.data.as_ptr().add(offset) as *const T;
            ptr.read_unaligned()
        }
    }

    /// Read the block as a specific type mutably.
    pub fn read_as_mut<T: Copy>(&mut self) -> &mut T {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut T;
            &mut *ptr
        }
    }

    /// Read the block as a specific type mutably at a specific offset.
    pub fn read_offset_as_mut<T: Copy>(&mut self, offset: usize) -> &mut T {
        unsafe {
            let ptr = self.data.as_mut_ptr().add(offset) as *mut T;
            &mut *ptr
        }
    }

    /// Write data to the block starting at a specific offset.
    pub fn write_offset(&mut self, offset: usize, data: &[u8], len: usize) {
        let end = offset + len;
        if end <= self.data.len() {
            let slice_end = len.min(data.len());
            self.data[offset..end].copy_from_slice(&data[..slice_end]);
        } else {
            panic!("Write would overflow the block buffer");
        }
    }

    
    /// Flush the cache to disk
    #[cfg(feature = "blockcache")]
    pub fn flush(&self, block_device: Arc<dyn BlockDevice>) {
        let mut cache = self.cache.lock();
        for (offset, block) in cache.map.iter() {
            block_device.write_offset(*offset, &block.data);
        }
        cache.map.clear();
        cache.queue.clear();
    }

    #[cfg(not(feature = "blockcache"))]
    pub fn flush(&self, _block_device: Arc<dyn BlockDevice>) {
        // Do nothing if blockcache feature is not enabled
    }
}

impl Block {
    #[cfg(feature = "blockcache")]
    pub fn sync_blk_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        block_device.write_offset(self.disk_offset, &self.data);
        let mut cache = self.cache.lock();
        cache.put(self.disk_offset, self.clone());
    }

    #[cfg(not(feature = "blockcache"))]
    pub fn sync_blk_to_disk(&self, block_device: Arc<dyn BlockDevice>) {
        block_device.write_offset(self.disk_offset, &self.data);
    }
}

// 定义 LRU 缓存结构
type LruCacheKey = usize;
type LruCacheValue = Block;

#[cfg(feature = "blockcache")]
pub struct LruCache {
    pub capacity: usize,
    pub queue: VecDeque<LruCacheKey>,
    pub map: BTreeMap<LruCacheKey, LruCacheValue>,
}

#[cfg(feature = "blockcache")]
impl LruCache {
    pub fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            queue: VecDeque::new(),
            map: BTreeMap::new(),
        }
    }

    pub fn get(&mut self, key: LruCacheKey) -> Option<&LruCacheValue> {
        if let Some(value) = self.map.get(&key) {
            if let Some(index) = self.queue.iter().position(|&k| k == key) {
                self.queue.remove(index);
            }
            self.queue.push_front(key);
            Some(value)
        } else {
            None
        }
    }

    pub fn put(&mut self, key: LruCacheKey, value: LruCacheValue) {
        if self.map.contains_key(&key) {
            if let Some(index) = self.queue.iter().position(|&k| k == key) {
                self.queue.remove(index);
            }
        } else if self.map.len() >= self.capacity {
            if let Some(oldest_key) = self.queue.pop_back() {
                self.map.remove(&oldest_key);
            }
        }
        self.queue.push_front(key);
        self.map.insert(key, value);
    }
}

use super::*;

impl Block {
    pub fn extent_block_csum_set(
        &mut self,
        block_device: Arc<dyn BlockDevice>,
        sb: &Ext4Superblock,
        inode_num: u32,
        ino_gen: u32,
        new_header: &Ext4ExtentHeader,
    ) {
        let last = new_header.max_entries_count;

        let tail_offset = BLOCK_SIZE - size_of::<Ext4ExtentTail>();

        // let tail_offset = size_of::<Ext4ExtentHeader>()
        //     + size_of::<Ext4Extent>() * new_header.max_entries_count as usize;
        let mut tail: Ext4ExtentTail = *self.read_offset_as_mut(tail_offset);

        log::info!("tail offset {:x?}", tail_offset);
        tail.tail_set_csum(sb, &self.data, inode_num, ino_gen);
    }
}