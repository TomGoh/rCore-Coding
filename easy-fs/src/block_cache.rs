use core::{
    alloc::Layout,
    mem::ManuallyDrop,
    num::NonZero,
    ptr::{addr_of, addr_of_mut},
};

use crate::{BLOCK_SZ, block_dev::BlockDevice};
use alloc::{boxed::Box, slice, sync::Arc, vec::Vec};
use lazy_static::*;
use lru::LruCache;
use spin::Mutex;

const BLOCK_CACHE_SIZE: usize = 16;

lazy_static! {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

struct CacheData(ManuallyDrop<Box<[u8; BLOCK_SZ]>>);

/// 块缓存
pub struct BlockCache {
    /// 缓存的数据，位于内存中，大小为 BLOCK_SZ
    cache: CacheData,
    /// 缓存对应的块设备中的块号
    block_id: usize,
    /// 块设备的引用，可以通过它读写对应实际块设备上的块
    block_device: Arc<dyn BlockDevice>,
    /// 缓存是否被修改过
    modified: bool,
}

pub struct BlockCacheManager {
    queue: LruCache<usize, Arc<Mutex<BlockCache>>>,
}

impl CacheData {
    pub fn new() -> Self {
        let data = unsafe {
            let raw = alloc::alloc::alloc(Self::layout());
            Box::from_raw(raw as *mut [u8; BLOCK_SZ])
        };
        Self(ManuallyDrop::new(data))
    }

    fn layout() -> Layout {
        Layout::from_size_align(BLOCK_SZ, BLOCK_SZ).unwrap()
    }
}

impl Drop for CacheData {
    fn drop(&mut self) {
        let ptr = self.0.as_mut_ptr();
        unsafe { alloc::alloc::dealloc(ptr, Self::layout()) };
    }
}

impl AsRef<[u8]> for CacheData {
    fn as_ref(&self) -> &[u8] {
        let ptr = self.0.as_ptr() as *const u8;
        unsafe { slice::from_raw_parts(ptr, BLOCK_SZ) }
    }
}

impl AsMut<[u8]> for CacheData {
    fn as_mut(&mut self) -> &mut [u8] {
        let ptr = self.0.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts_mut(ptr, BLOCK_SZ) }
    }
}

impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = CacheData::new();
        block_device.read_block(block_id, cache.as_mut());
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    fn addr_of_offset(&self, offset: usize) -> *const u8 {
        addr_of!(self.cache.as_ref()[offset])
    }

    fn addr_of_offset_mut(&mut self, offset: usize) -> *mut u8 {
        addr_of_mut!(self.cache.as_mut()[offset])
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset) as *const T;
        unsafe { &*addr }
    }

    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset_mut(offset) as *mut T;
        self.modified = true;
        unsafe { &mut *addr }
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.block_device
                .write_block(self.block_id, self.cache.as_ref());
            self.modified = false;
        }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V
    where
        T: Sized,
    {
        f(self.get_ref(offset))
    }

    pub fn write<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V
    where
        T: Sized,
    {
        f(self.get_mut(offset))
    }

    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V
    where
        T: Sized,
    {
        f(self.get_mut(offset))
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: LruCache::new(NonZero::new(BLOCK_CACHE_SIZE).unwrap()),
        }
    }

    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(cache) = self.queue.get(&block_id) {
            cache.clone()
        } else {
            if self.queue.len() == self.queue.cap().get() {
                if Arc::strong_count(self.queue.peek_lru().unwrap().1) == 1 {
                    self.queue.pop_lru();
                } else {
                    let mut skipped = Vec::with_capacity(self.queue.cap().get());
                    let mut evicted = false;
                    while let Some((block_id, cache)) = self.queue.pop_lru() {
                        if Arc::strong_count(&cache) == 1 {
                            evicted = true;
                            break;
                        } else {
                            skipped.push((block_id, cache));
                        }
                    }
                    for (block_id, cache) in skipped.into_iter().rev() {
                        self.queue.put(block_id, cache);
                    }
                    if !evicted {
                        panic!("No cache entry is evictable!");
                    }
                }
            }
            let cache = Arc::new(Mutex::new(BlockCache::new(block_id, block_device)));
            self.queue.put(block_id, cache.clone());
            cache
        }
    }
}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

/// Sync all block cache to block device
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
