use crate::{BLOCK_SZ, BlockDevice, block_cache::get_block_cache};
use alloc::sync::Arc;

/// 单个缓存下来的位图磁盘块，内部包含 64 个 `u64`，共 4096 个可分配的比特。
type BitmapBlock = [u64; 64];
/// 一个磁盘块可表示的比特数量（块大小乘以 8），即位图在块级别的跨度。
const BLOCK_BITS: usize = BLOCK_SZ * 8;

/// 位图结构，用于跟踪磁盘数据块或 iNode 节点的使用情况并提供分配/释放接口。
pub struct Bitmap {
    /// 位图在磁盘上的起始块号，对应位图数据在设备中的偏移。
    start_block_id: usize,
    /// 位图连续占据的块数，决定能够管理的比特总量。
    blocks: usize,
}

impl Bitmap {
    /// 根据位图的起始块号和覆盖块数构造一个新的位图管理器。
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// 在线性扫描位图数据时寻找第一个空闲块，并将其标记为已占用。
    /// 成功返回对应的全局块号，若位图已满则返回 `None`。
    ///
    /// 该函数会线性扫描位图所覆盖的所有块，查找第一个未被占用的比特位。
    /// 针对每一个位图块，它会逐个检查其中的 `u64`，寻找第一个不全为 1 的 `u64`。
    /// 一旦找到这样的 `u64`，就能确定其中至少有一个空闲的比特位。
    ///
    /// 参数：
    /// - `block_device`：实现了 `BlockDevice` 接口的块设备实例，用于读写位图数据。
    ///
    /// 返回值：
    /// - `Option<usize>`：成功时返回分配的块号，失败时返回 `None`。
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            // 逐个访问位图所在的磁盘块，将其缓存在内存中进行修改。
            let pos = get_block_cache(block_id + self.start_block_id, Arc::clone(block_device))
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    // 找到第一个非全 1 的 `u64`，其中必定存在空闲的比特位。
                    if let Some((bits64_pos, inner_pos)) = bitmap_block
                        .iter()
                        .enumerate()
                        // 查找第一个不全为 1 的 `u64`，当 `bits64` 不是每一个比特都为 1 时即满足条件，也就是不等于 `u64::MAX`
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                        .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                    {
                        // 将对应的比特位置 1，并返回对应的全局块号。
                        bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                        Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos)
                    } else {
                        None
                    }
                });

            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    /// 根据给定的块号（比特索引）释放磁盘块，并在位图中清除此比特。
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        // 校验要释放的比特是否仍在当前位图的管理范围内。
        assert!(bit < self.blocks * BLOCK_BITS);
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // 被释放的位置应该原本为 1，否则表示重复释放或越界。
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }

    /// 返回该位图能够管理的最大块数量，用于进行容量边界判断。
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}

/// 将全局比特编号拆解为三元组：所在的位图块、块内的 `u64` 索引以及比特位位置。
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    // 计算当前比特对应的位图块编号。
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}
