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

/// 块缓存总容量；在缓存满时会触发最近最少使用 (LRU) 淘汰。
const BLOCK_CACHE_SIZE: usize = 16;

lazy_static! {
    /// 块缓存管理器的全局单例，负责调度所有块缓存实例。
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

/// 底层缓存数据的封装类型，托管 `BLOCK_SZ` 字节的缓冲区并延迟回收。
///
/// 通过 `ManuallyDrop` 包装 `Box<[u8; BLOCK_SZ]>`，避免结构体在移动或销毁时被
/// 提前释放，改为由自定义的 `Drop` 实现统一回收，以便精确控制内存生命周期。
struct CacheData(ManuallyDrop<Box<[u8; BLOCK_SZ]>>);

/// 块缓存实体，维护一份磁盘块的内存镜像并提供读写接口。
pub struct BlockCache {
    /// 缓存在内存中的原始数据，长度固定为 `BLOCK_SZ` 字节。
    cache: CacheData,
    /// 当前缓存所对应的磁盘块号。
    block_id: usize,
    /// 底层块设备的引用，用于在必要时同步数据。
    block_device: Arc<dyn BlockDevice>,
    /// 是否对缓存内容做过修改，决定是否需要回写磁盘。
    modified: bool,
}

/// 块缓存管理器，采用 LRU 算法管理有限数量的块缓存。
pub struct BlockCacheManager {
    queue: LruCache<usize, Arc<Mutex<BlockCache>>>,
}

impl CacheData {
    /// 分配一片符合块大小和对齐要求的内存区域，并构造 `CacheData`。
    ///
    /// 该函数通过 `alloc::alloc` 直接向分配器申请内存，确保返回的缓冲区大小与
    /// 对齐均满足块缓存的要求。
    ///
    /// 参数：
    /// - `self`：静态关联函数，无需参数。
    ///
    /// 返回值：
    /// - `Self`：承载分配结果的 `CacheData` 实例。
    pub fn new() -> Self {
        let data = unsafe {
            let raw = alloc::alloc::alloc(Self::layout());
            Box::from_raw(raw as *mut [u8; BLOCK_SZ])
        };
        Self(ManuallyDrop::new(data))
    }

    /// 返回创建块缓存时所需的内存布局信息。
    ///
    /// 该函数封装了块大小和对齐的组合，供内存分配与释放时统一使用。
    ///
    /// 参数：
    /// - `self`：静态关联函数，无需参数。
    ///
    /// 返回值：
    /// - `Layout`：描述块缓存内存需求的布局对象。
    fn layout() -> Layout {
        Layout::from_size_align(BLOCK_SZ, BLOCK_SZ).unwrap()
    }
}

impl Drop for CacheData {
    /// 在 `CacheData` 生命周期结束时释放底层缓冲区。
    ///
    /// `ManuallyDrop` 会阻止 `Box<[u8; BLOCK_SZ]>` 在离开作用域时自动析构，
    /// 因此需要在此手动调用分配器的 `dealloc`，保证内存被正常回收。
    fn drop(&mut self) {
        let ptr = self.0.as_mut_ptr();
        unsafe { alloc::alloc::dealloc(ptr, Self::layout()) };
    }
}

impl AsRef<[u8]> for CacheData {
    /// 以只读切片形式暴露底层缓存数据，便于外部读取。
    ///
    /// 该实现会根据缓存指针构造长度为 `BLOCK_SZ` 的字节切片，不会改变内部状态。
    fn as_ref(&self) -> &[u8] {
        let ptr = self.0.as_ptr();
        unsafe { slice::from_raw_parts(ptr, BLOCK_SZ) }
    }
}

impl AsMut<[u8]> for CacheData {
    /// 以可写切片形式暴露底层缓存数据，用于修改块内容。
    ///
    /// 返回的切片与底层缓冲区共享内存，调用者可直接在其上进行写操作。
    fn as_mut(&mut self) -> &mut [u8] {
        let ptr = self.0.as_mut_ptr();
        unsafe { slice::from_raw_parts_mut(ptr, BLOCK_SZ) }
    }
}

impl BlockCache {
    /// 依据块号从块设备中读取数据，并构造一个块缓存实例。
    ///
    /// 该函数会为缓存分配一片内存，并立即从块设备中读取对应块的数据填充缓存，
    /// 初始状态下 `modified` 标记为 `false`，表示尚未修改。
    ///
    /// 参数：
    /// - `block_id`：目标磁盘块号。
    /// - `block_device`：实现 `BlockDevice` 的块设备引用，用于执行读操作。
    ///
    /// 返回值：
    /// - `Self`：初始化完成的块缓存实例。
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

    /// 从指定偏移处以引用形式读取类型 `T` 的数据。
    ///
    /// 调用者需保证偏移与类型尺寸合规，函数内部会执行边界检查并返回指向缓存中
    /// 对应位置的引用，不会触发修改标记。
    ///
    /// 参数：
    /// - `offset`：数据在块内的字节偏移。
    ///
    /// 返回值：
    /// - `&T`：指向块缓存内该位置数据的共享引用。
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset) as *const T;
        unsafe { &*addr }
    }

    /// 从指定偏移处获取可变引用，并将缓存标记为已修改。
    ///
    /// 在返回引用之前会检查访问是否越界，随后标记 `modified = true`，以便在缓存
    /// 被回收或显式同步时将改动写回块设备。
    ///
    /// 参数：
    /// - `offset`：数据在块内的字节偏移。
    ///
    /// 返回值：
    /// - `&mut T`：指向块缓存内该位置数据的可变引用。
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

    /// 将缓存内容同步回块设备，若未修改则不执行任何操作。
    ///
    /// 该函数检查 `modified` 标记，仅当缓存数据被更改过时才会触发写回，并在写回
    /// 成功后清除修改标记，避免重复写入。
    ///
    /// 参数：
    /// - `self`：当前块缓存的可变引用。
    ///
    /// 返回值：
    /// - `()`：无返回值。
    pub fn sync(&mut self) {
        if self.modified {
            self.block_device
                .write_block(self.block_id, self.cache.as_ref());
            self.modified = false;
        }
    }

    /// 以共享方式读取缓存中类型 `T` 的数据，并对其执行闭包 `f`。
    ///
    /// 该接口是 `get_ref` 的函数式封装，在闭包执行后返回其结果，可避免在调用方
    /// 暴露底层引用，提高封装性。
    ///
    /// 参数：
    /// - `offset`：数据在块内的字节偏移。
    /// - `f`：对读取出的引用执行操作的闭包，类型为 `FnOnce(&T) -> V`。
    ///
    /// 返回值：
    /// - `V`：闭包执行后的返回结果。
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V
    where
        T: Sized,
    {
        f(self.get_ref(offset))
    }

    /// 以可变方式访问缓存中类型 `T` 的数据，并对其执行闭包 `f`。
    ///
    /// 函数内部会调用 `get_mut` 获取可变引用，因此会自动标记缓存为已修改。
    ///
    /// 参数：
    /// - `offset`：数据在块内的字节偏移。
    /// - `f`：对可变引用执行操作的闭包，类型为 `FnOnce(&mut T) -> V`。
    ///
    /// 返回值：
    /// - `V`：闭包执行后的返回结果。
    #[allow(dead_code)]
    pub fn write<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V
    where
        T: Sized,
    {
        f(self.get_mut(offset))
    }

    /// 以可变方式访问缓存中类型 `T` 的数据，并返回闭包 `f` 的执行结果。
    ///
    /// 该函数语义与 `write` 相同，主要用于语义表达清晰的场景，内部同样调用
    /// `get_mut` 并标记缓存修改。
    ///
    /// 参数：
    /// - `offset`：数据在块内的字节偏移。
    /// - `f`：对可变引用执行操作的闭包，类型为 `FnOnce(&mut T) -> V`。
    ///
    /// 返回值：
    /// - `V`：闭包执行后的返回结果。
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
    /// 创建块缓存管理器，并初始化 LRU 队列容量。
    ///
    /// 该函数设置缓存容量上限，后续当缓存数量达到上限时会根据 LRU 策略淘汰。
    ///
    /// 参数：
    /// - `self`：静态关联函数，无需参数。
    ///
    /// 返回值：
    /// - `Self`：新的 `BlockCacheManager` 实例。
    pub fn new() -> Self {
        Self {
            queue: LruCache::new(NonZero::new(BLOCK_CACHE_SIZE).unwrap()),
        }
    }

    /// 获取或创建指定块号的缓存实例，必要时执行 LRU 淘汰。
    ///
    /// 若缓存中已存在所需块，则直接返回其引用；否则会在容量受限时寻找可以淘汰
    /// 的缓存条目，若所有缓存均被其它引用持有，则会触发 panic，以提示无法继续
    /// 分配新缓存。
    ///
    /// 参数：
    /// - `block_id`：目标磁盘块号。
    /// - `block_device`：块设备引用，缺少缓存时需要用它加载磁盘数据。
    ///
    /// 返回值：
    /// - `Arc<Mutex<BlockCache>>`：线程安全的块缓存引用。
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

/// 获取指定块号的缓存，内部复用全局块缓存管理器。
///
/// 该函数对全局管理器加锁后，转调 `BlockCacheManager::get_block_cache`，从而实现
/// 多线程场景下的缓存共享。
///
/// 参数：
/// - `block_id`：目标磁盘块号。
/// - `block_device`：块设备引用，缺少缓存时用于加载数据。
///
/// 返回值：
/// - `Arc<Mutex<BlockCache>>`：线程安全的块缓存引用。
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

/// 将所有缓存的修改写回块设备，确保数据落盘。
///
/// 该函数遍历 LRU 队列中的每个缓存，逐个加锁并调用 `sync` 完成写回，在文件系统
/// 关闭或需要强制落盘时使用。
///
/// 参数：
/// - `self`：自由函数，无需参数。
///
/// 返回值：
/// - `()`：无返回值。
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
