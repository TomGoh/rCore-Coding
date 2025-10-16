use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    BLOCK_SZ, BlockDevice, Inode,
    bitmap::Bitmap,
    block_cache_sync_all, get_block_cache,
    layout::{DiskInode, DiskInodeType, SuperBlock},
};

type DataBlock = [u8; BLOCK_SZ];

/// 简化版文件系统核心结构，负责分配磁盘块与 inode 并维护整体布局。
pub struct EasyFileSystem {
    /// 底层块设备引用，用于读写实际的磁盘块数据。
    pub block_device: Arc<dyn BlockDevice>,
    /// inode 使用情况的位图，负责跟踪哪些 inode 已经被占用。
    pub inode_bitmap: Bitmap,
    /// 数据块使用情况的位图，负责管理普通数据块的分配与回收。
    pub data_bitmap: Bitmap,
    /// inode 区域在磁盘上的起始块号，便于定位 inode 所在块。
    inode_area_start_block: u32,
    /// 数据区在磁盘上的起始块号，用于计算数据块的全局编号。
    data_area_start_block: u32,
}

impl EasyFileSystem {
    /// 创建并初始化一个全新的 `EasyFileSystem`。
    ///
    /// 该函数会根据给定的块数量与位图大小，计算 inode 区域和数据区域的布局，
    /// 并依次执行以下步骤：
    /// 1. 为 inode 位图与数据位图创建管理结构；
    /// 2. 清零所有磁盘块，保证后续读取的数据初始为 0；
    /// 3. 写入超级块元数据，记录文件系统的布局信息；
    /// 4. 创建根目录 `/` 对应的 inode。完成后会立即同步缓存，确保磁盘一致。
    ///
    /// 参数：
    /// - `block_device`：实现 `BlockDevice` 的设备引用，作为文件系统的存储介质。
    /// - `total_blocks`：设备总块数，用于计算各个区域的大小。
    /// - `inode_bitmap_blocks`：分配给 inode 位图的块数，用于限定 inode 数量。
    ///
    /// 返回值：
    /// - `Arc<Mutex<Self>>`：受互斥锁保护的文件系统句柄，便于并发访问。
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // 首先，根据传入的 inode 位图块数计算 inode 数量与区域大小。
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.maximum();
        let inode_area_blocks =
            (inode_num * core::mem::size_of::<DiskInode>()).div_ceil(BLOCK_SZ) as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;

        // 其次，计算数据区的布局：位图块数与数据块数。
        // 注意：数据区块数需要扣除超级块、inode 位图块和 inode 区块，同时由于数据区块
        // 也需要位图块来管理，因此需要额外加上位图块数。
        // 设数据区块数为 x，则位图块数为 ceil(x / 4096)，因此有：
        // x + ceil(x / 4096) = total_blocks - 1 - inode_total_blocks
        // 近似可解为：
        // x + (x / 4096) = total_blocks - 1 - inode_total_blocks
        // x * 4097 / 4096 = total_blocks - 1 - inode_total_blocks
        // x = (total_blocks - 1 - inode_total_blocks) * 4096 / 4097
        // 由此可得数据区块数与位图块数的计算公式。
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        let data_bitmap_blocks = data_total_blocks.div_ceil(4097);
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = Bitmap::new(
            (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );

        // 最后，构造文件系统实例并依次初始化各个部分。
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // 清空全部磁盘块，避免读取到旧数据。
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // 初始化超级块，记录文件系统基础布局信息。
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        // 创建根目录 inode，并立即写回持久化。
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }

    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }

    /// 分配一个新的 inode，并返回其编号。
    ///
    /// 该函数调用 inode 位图的分配逻辑，找到第一个空闲的 inode 并将其标记为
    /// 已使用；若位图已满会触发 panic。
    ///
    /// 参数：
    /// - `self`：文件系统的可变引用。
    ///
    /// 返回值：
    /// - `u32`：成功分配的 inode 编号。
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// 分配一个新的数据块，并返回其全局块号。
    ///
    /// 函数内部会从数据位图中找到空闲块，将其标记为已占用，然后根据数据区的
    /// 起始块号计算真实的磁盘块编号；若位图已满会触发 panic。
    ///
    /// 参数：
    /// - `self`：文件系统的可变引用。
    ///
    /// 返回值：
    /// - `u32`：成功分配的数据块的全局块号。
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    /// 根据 inode 编号定位到对应的磁盘块及其偏移。
    ///
    /// 该函数会计算每个块可容纳的 inode 数量，并以此推导给定 inode 所在的块号
    /// 与块内偏移，便于后续读写具体的磁盘 inode 结构。
    ///
    /// 参数：
    /// - `inode_id`：目标 inode 的编号。
    ///
    /// 返回值：
    /// - `(u32, usize)`：包含 inode 所在的块号以及在块内的字节偏移。
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inode_per_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inode_per_block;
        (block_id, (inode_id % inode_per_block) as usize * inode_size)
    }

    /// 释放指定的数据块并清零其内容。
    ///
    /// 函数首先验证待释放的块号位于数据区范围内，随后将缓存中的数据写成全 0，
    /// 最后调用数据位图的 `dealloc` 将相应比特清除，表示该块重新变为可用状态。
    ///
    /// 参数：
    /// - `data_block_id`：需要释放的全局数据块号。
    ///
    /// 返回值：
    /// - `()`：无返回值。
    pub fn dealloc_data(&mut self, data_block_id: u32) {
        assert!(data_block_id >= self.data_area_start_block);
        get_block_cache(data_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (data_block_id - self.data_area_start_block) as usize,
        );
    }

    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);

        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }
}
