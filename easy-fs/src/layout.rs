use super::{BLOCK_SZ, BlockDevice, get_block_cache};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result};

/// Magic number for sanity check
const EFS_MAGIC: u32 = 0x3b800001;
/// The max number of direct inodes
const INODE_DIRECT_COUNT: usize = 28;
/// The max length of inode name
const NAME_LENGTH_LIMIT: usize = 27;
/// The max number of indirect1 inodes
const INODE_INDIRECT1_COUNT: usize = BLOCK_SZ / 4;
/// The max number of indirect2 inodes
const INODE_INDIRECT2_COUNT: usize = INODE_INDIRECT1_COUNT * INODE_INDIRECT1_COUNT;
/// The upper bound of direct inode index
const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
/// The upper bound of indirect1 inode index
const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_INDIRECT1_COUNT;
/// The upper bound of indirect2 inode indexs
#[allow(unused)]
const INDIRECT2_BOUND: usize = INDIRECT1_BOUND + INODE_INDIRECT2_COUNT;
/// Size of a directory entry
pub const DIRENT_SZ: usize = 32;

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

type IndirectBlock = [u32; BLOCK_SZ / 4];
type DataBlock = [u8; BLOCK_SZ];

/// Layout of a Disk Inode :
/// ```c
/// DiskInode
/// ├── direct[28]           → 28 data blocks directly
/// ├── indirect1            → 1 indirect block
/// │ └── [128 entries]    → 128 data blocks
/// └── indirect2            → 1 indirect block
///     └── [128 entries]    → each points to another indirect block
///         ├── indirect1_0 → [128 entries] → 128 data blocks
///         ├── indirect1_1 → [128 entries] → 128 data blocks
///         ├── ...
///         └── indirect1_127 → [128 entries] → 128 data blocks
/// ```
#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}

#[repr(C)]
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_number: u32,
}

impl Debug for SuperBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("SuperBlock")
            .field("total_blocks", &self.total_blocks)
            .field("inode_bitmap_blocks", &self.inode_bitmap_blocks)
            .field("inode_area_blocks", &self.inode_area_blocks)
            .field("data_bitmap_blocks", &self.data_bitmap_blocks)
            .field("data_area_blocks", &self.data_area_blocks)
            .finish()
    }
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

impl DiskInode {
    pub fn initialize(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct.iter_mut().for_each(|v| *v = 0);
        self.indirect1 = 0;
        self.indirect2 = 0;
        self.type_ = type_;
    }

    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }

    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }

    pub fn _data_blocks(size: u32) -> u32 {
        size.div_ceil(BLOCK_SZ as u32)
    }

    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }

    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks;
        // indirect1
        if data_blocks > INODE_DIRECT_COUNT {
            total += 1;
        }
        // indirect2
        if data_blocks > INDIRECT1_BOUND {
            total += 1;
            // sub indirect1
            total += (data_blocks - INDIRECT1_BOUND).div_ceil(INODE_INDIRECT1_COUNT);
        }
        total as u32
    }

    pub fn blocks_num_needed_for_new_size(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INDIRECT1_BOUND;
            let indirect1 = get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect2: &IndirectBlock| {
                    indirect2[last / INODE_INDIRECT1_COUNT]
                });
            get_block_cache(indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect1: &IndirectBlock| {
                    indirect1[last % INODE_INDIRECT1_COUNT]
                })
        }
    }

    /// 增加 inode 的大小，分配新的数据块
    ///
    /// 采用分层块分配策略：直接块(28) → 一级间接(128) → 二级间接(16384)
    /// 设计目标：小文件快速访问，大文件仍有合理性能
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        // ========== 阶段 1: 初始化状态 ==========
        // 计算当前大小需要的块数（起点）
        let mut current_blocks_count = self.data_blocks();
        // 先更新文件大小，因为 data_blocks() 依赖于 self.size
        self.size = new_size;
        // 计算新大小需要的块数（终点）
        let mut total_blocks_count = self.data_blocks();
        // 将新块列表转为迭代器，方便逐个消费
        let mut new_blocks = new_blocks.into_iter();

        // ========== 阶段 2: 填充直接块 (索引 0-27) ==========
        // 直接块存储在 inode 内部，访问最快（零间接开销）
        // min() 确保不越界：如果需要超过 28 块，只填充前 28 个
        while current_blocks_count < total_blocks_count.min(INODE_DIRECT_COUNT as u32) {
            // 将新块号直接写入 direct 数组
            self.direct[current_blocks_count as usize] = new_blocks.next().unwrap();
            current_blocks_count += 1;
        }

        // ========== 阶段 3: 一级间接块 (索引 28-155) ==========
        // 检查是否需要进入一级间接（需要 > 28 块）
        if total_blocks_count > INODE_DIRECT_COUNT as u32 {
            // 惰性分配：只在恰好从直接块转换到间接块时，才分配 indirect1 块本身
            // 如果文件之前已经有 indirect1，这里不会重复分配
            if current_blocks_count == INODE_DIRECT_COUNT as u32 {
                self.indirect1 = new_blocks.next().unwrap();
            }
            // 坐标变换：从全局坐标转换为 indirect1 局部坐标
            // 全局块 28 → indirect1 索引 0，全局块 155 → indirect1 索引 127
            current_blocks_count -= INODE_DIRECT_COUNT as u32;
            total_blocks_count -= INODE_DIRECT_COUNT as u32;
        } else {
            // 如果不需要 indirect1，直接返回
            return;
        }

        // 填充 indirect1 块中的条目
        // indirect1 是一个包含 128 个 u32 的块，每个 u32 指向一个数据块
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock() // 加锁保证线程安全
            .modify(0, |indirect1: &mut IndirectBlock| {
                // 在 indirect1 块内填充数据块指针
                while current_blocks_count < total_blocks_count.min(INODE_INDIRECT1_COUNT as u32) {
                    indirect1[current_blocks_count as usize] = new_blocks.next().unwrap();
                    current_blocks_count += 1;
                }
            }); // modify 标记块为脏，延迟写回磁盘（写回缓存）

        // ========== 阶段 4: 二级间接块 (索引 156-16539) ==========
        // 检查是否需要进入二级间接（需要 > 156 块）
        if total_blocks_count > INODE_INDIRECT1_COUNT as u32 {
            // 惰性分配：只在恰好从一级间接转换到二级间接时，才分配 indirect2 块本身
            if current_blocks_count == INODE_INDIRECT1_COUNT as u32 {
                self.indirect2 = new_blocks.next().unwrap();
            }
            // 坐标变换：从 indirect1 坐标转换为 indirect2 局部坐标
            current_blocks_count -= INODE_INDIRECT1_COUNT as u32;
            total_blocks_count -= INODE_INDIRECT1_COUNT as u32;
        } else {
            // 如果不需要二级间接，文件大小在中等范围，直接返回
            return;
        }

        // 二维坐标转换：indirect2 是 128×128 的矩阵结构
        // a0/a1: 行索引（第几个一级间接块）
        // b0/b1: 列索引（该一级间接块内的第几个条目）
        let mut a0 = current_blocks_count as usize / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks_count as usize % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks_count as usize / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks_count as usize % INODE_INDIRECT1_COUNT;

        // 填充 indirect2 结构
        // indirect2 块包含 128 个 u32，每个指向一个一级间接块
        // 每个一级间接块又包含 128 个 u32，每个指向一个数据块
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock() // 锁定 indirect2 块
            .modify(0, |indirect2: &mut IndirectBlock| {
                // 行优先遍历：逐行填充，每行填充完再移到下一行
                // (a0 < a1): 还没到最后一行，填充整行
                // (a0 == a1 && b0 < b1): 在最后一行，只填充到 b1 列
                while (a0 < a1) || (a0 == a1 && b0 < b1) {
                    // 稀疏分配：只在开始新行时（b0 == 0）才分配新的一级间接块
                    // 这样避免浪费：如果只需要几个块，不会分配 128 个一级间接块
                    if b0 == 0 {
                        indirect2[a0] = new_blocks.next().unwrap();
                    }

                    // 嵌套访问：读取一级间接块，填充其中的数据块指针
                    // 在外层 modify 闭包内访问 indirect2[a0]，保证一致性
                    get_block_cache(indirect2[a0] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            // 填充当前位置的数据块号
                            indirect1[b0] = new_blocks.next().unwrap();
                        });

                    // 移动到下一个位置（列优先，类似 C 语言二维数组遍历）
                    b0 += 1;
                    // 换行：如果当前行填满（128 列），移到下一行
                    if b0 == INODE_INDIRECT1_COUNT {
                        b0 = 0;
                        a0 += 1;
                    }
                }
            });
    }

    /// 清空 inode，回收所有数据块
    ///
    /// 与 increase_size 相反，该函数释放文件占用的所有数据块。
    /// 采用逆向遍历策略：直接块 → 一级间接 → 二级间接
    ///
    /// # 返回值
    /// 返回所有被释放的块号列表，调用者需要将这些块标记为可用
    ///
    /// # 设计原理
    /// - 收集所有块号（包括数据块和间接块本身）
    /// - 清零 inode 中的所有指针
    /// - 将 size 设置为 0
    /// - 返回块号列表供位图管理器回收
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        // ========== 初始化 ==========
        // 存储所有需要回收的块号
        let mut v: Vec<u32> = Vec::new();
        // 获取当前文件使用的数据块数量
        let mut data_block_count = self.data_blocks() as usize;
        // 先将文件大小设为 0（逻辑上已清空）
        self.size = 0;
        // 当前处理的块索引
        let mut current_block_count = 0usize;

        // ========== 阶段 1: 回收直接块 (索引 0-27) ==========
        // 遍历所有使用的直接块
        while current_block_count < data_block_count.min(INODE_DIRECT_COUNT) {
            // 将块号添加到回收列表
            v.push(self.direct[current_block_count]);
            // 清零该直接块指针
            self.direct[current_block_count] = 0;
            current_block_count += 1;
        }

        // ========== 阶段 2: 回收一级间接块 ==========
        // 检查是否使用了一级间接块
        if data_block_count > INODE_DIRECT_COUNT {
            // 将 indirect1 块本身也加入回收列表（它是结构块，不是数据块）
            v.push(self.indirect1);
            // 坐标变换：转换到 indirect1 局部坐标系
            data_block_count -= INODE_DIRECT_COUNT;
            current_block_count = 0;
        } else {
            // 如果只使用了直接块，直接返回
            return v;
        }

        // 读取 indirect1 块，回收其中指向的所有数据块
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                // 遍历 indirect1 中所有使用的条目
                while current_block_count < data_block_count.min(INODE_INDIRECT1_COUNT) {
                    // 将数据块号添加到回收列表
                    v.push(indirect1[current_block_count]);
                    // 注意：这里不需要清零 indirect1 的条目，因为整个 indirect1 块都会被回收
                    current_block_count += 1;
                }
            });
        // 清零 indirect1 指针（该块已在上面加入回收列表）
        self.indirect1 = 0;

        // ========== 阶段 3: 回收二级间接块 ==========
        // 检查是否使用了二级间接块
        if data_block_count > INODE_INDIRECT1_COUNT {
            // 将 indirect2 块本身也加入回收列表
            v.push(self.indirect2);
            // 坐标变换：转换到 indirect2 局部坐标系
            data_block_count -= INODE_INDIRECT1_COUNT;
        } else {
            // 如果只使用到一级间接，直接返回
            return v;
        }

        // 断言：数据块数量不应超过二级间接的最大容量
        assert!(data_block_count <= INODE_INDIRECT2_COUNT);

        // 二维坐标计算：将线性块数转换为 (行, 列) 坐标
        // a1: 完整的一级间接块数量（完整的行）
        // b1: 最后一个不完整的一级间接块中的数据块数量（最后一行的列数）
        let a1 = data_block_count / INODE_INDIRECT1_COUNT;
        let b1 = data_block_count % INODE_INDIRECT1_COUNT;

        // 读取 indirect2 块
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                // ===== 处理完整的一级间接块 =====
                // 遍历前 a1 个一级间接块（每个都装满了 128 个数据块）
                for entry in indirect2.iter_mut().take(a1) {
                    // 将一级间接块本身加入回收列表
                    v.push(*entry);
                    // 读取该一级间接块，回收其中的所有数据块
                    get_block_cache(*entry as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            // 遍历该一级间接块中的所有 128 个数据块
                            for entry in indirect1.iter() {
                                v.push(*entry);
                            }
                        });
                }
                // ===== 处理最后一个不完整的一级间接块 =====
                // 如果存在部分填充的一级间接块（b1 > 0）
                if b1 > 0 {
                    // 将该一级间接块本身加入回收列表
                    v.push(indirect2[a1]);
                    // 读取该一级间接块，只回收前 b1 个数据块
                    get_block_cache(indirect2[a1] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            // 只遍历前 b1 个条目（部分填充）
                            for entry in indirect1.iter().take(b1) {
                                v.push(*entry);
                            }
                        });
                    // 注意：不需要清零 indirect2[a1]，因为整个 indirect2 块都会被回收
                }
            });
        // 清零 indirect2 指针（该块已在上面加入回收列表）
        self.indirect2 = 0;

        // 返回所有需要回收的块号列表
        v
    }

    /// 从文件的指定偏移位置读取数据到缓冲区
    ///
    /// # 参数
    /// - `offset`: 文件内的字节偏移量（从文件开头计算）
    /// - `buf`: 目标缓冲区，读取的数据将写入此处
    /// - `block_device`: 块设备引用
    ///
    /// # 返回值
    /// 实际读取的字节数（可能小于 buf.len()，如果到达文件末尾）
    ///
    /// # 设计原理
    /// 文件数据跨越多个块存储，读取操作需要：
    /// 1. 将文件偏移量映射到块号和块内偏移
    /// 2. 逐块读取数据（处理跨块读取）
    /// 3. 通过 get_block_id() 处理间接块寻址
    /// 4. 利用块缓存减少磁盘 I/O
    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        // ========== 初始化读取范围 ==========
        // 当前读取位置（会随着读取推进而更新）
        let mut start = offset;
        // 计算读取的结束位置：不能超过文件大小
        // min() 确保不会读取超出文件范围的数据
        let end = (offset + buf.len()).min(self.size as usize);

        // ========== 边界检查 ==========
        // 如果起始位置已经超过或等于结束位置，无数据可读
        // 情况：offset >= 文件大小，或 buf.len() == 0
        if start >= end {
            return 0;
        }

        // ========== 初始化读取状态 ==========
        // 计算起始块号（start / BLOCK_SZ 得到第几个块）
        let mut start_block = start / BLOCK_SZ;
        // 已读取的字节数（用于跟踪 buf 中的写入位置）
        let mut read_size = 0usize;

        // ========== 逐块读取循环 ==========
        loop {
            // 计算当前块的结束位置
            // (start / BLOCK_SZ + 1) * BLOCK_SZ 得到当前块的下一个块的起始位置
            // 即当前块的结束位置（块边界对齐）
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            // 但不能超过整体读取的结束位置
            end_current_block = end_current_block.min(end);

            // 计算本次（当前块）要读取的字节数
            let block_read_size = end_current_block - start;

            // 在目标缓冲区中定位本次写入的位置
            // buf[read_size..] 表示从已读取部分之后开始写入
            let dest = &mut buf[read_size..read_size + block_read_size];

            // 通过 get_block_id 获取逻辑块号对应的物理块号
            // 这里会自动处理直接块、一级间接、二级间接的寻址
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock() // 加锁访问块缓存
            .read(0, |data_block: &DataBlock| {
                // 计算块内偏移：start % BLOCK_SZ 得到在当前块内的起始位置
                // 从块内偏移开始，读取 block_read_size 字节
                let src = &data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_read_size];
                // 将数据从块缓存复制到目标缓冲区
                dest.copy_from_slice(src);
            });

            // 更新已读取字节数
            read_size += block_read_size;

            // ========== 检查是否完成 ==========
            // 如果当前块的结束位置就是整体读取的结束位置，读取完成
            if end_current_block == end {
                break;
            }

            // ========== 移动到下一个块 ==========
            // 移动到下一个块
            start_block += 1;
            // 更新当前读取位置为下一个块的起始位置
            start = end_current_block;
        }

        // 返回实际读取的字节数
        read_size
    }

    /// 向文件的指定偏移位置写入数据
    ///
    /// # 参数
    /// - `offset`: 文件内的字节偏移量（从文件开头计算）
    /// - `buf`: 源缓冲区，包含要写入的数据
    /// - `block_device`: 块设备引用
    ///
    /// # 返回值
    /// 实际写入的字节数（可能小于 buf.len()，如果超过文件大小）
    ///
    /// # 设计原理
    /// 与 read_at 类似，但使用 modify() 而非 read()：
    /// 1. 将文件偏移量映射到块号和块内偏移
    /// 2. 逐块写入数据（处理跨块写入）
    /// 3. 通过 get_block_id() 处理间接块寻址
    /// 4. 使用 modify() 标记块为脏，触发写回缓存
    ///
    /// # 注意
    /// - 写入不能超过当前文件大小（需先调用 increase_size）
    /// - 使用 assert 确保不会越界写入
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        // ========== 初始化写入范围 ==========
        // 当前写入位置（会随着写入推进而更新）
        let mut start = offset;
        // 计算写入的结束位置：不能超过文件大小
        // min() 确保不会写入超出文件范围的数据
        let end = (offset + buf.len()).min(self.size as usize);
        // 断言：起始位置必须 <= 结束位置（防止逻辑错误）
        assert!(start <= end);

        // ========== 初始化写入状态 ==========
        // 计算起始块号（start / BLOCK_SZ 得到第几个块）
        let mut start_block = start / BLOCK_SZ;
        // 已写入的字节数（用于跟踪 buf 中的读取位置）
        let mut write_size = 0usize;

        // ========== 逐块写入循环 ==========
        loop {
            // 计算当前块的结束位置
            // (start / BLOCK_SZ + 1) * BLOCK_SZ 得到当前块的下一个块的起始位置
            // 即当前块的结束位置（块边界对齐）
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            // 但不能超过整体写入的结束位置
            end_current_block = end_current_block.min(end);

            // 计算本次（当前块）要写入的字节数
            let block_write_size = end_current_block - start;

            // 通过 get_block_id 获取逻辑块号对应的物理块号
            // 这里会自动处理直接块、一级间接、二级间接的寻址
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock() // 加锁访问块缓存
            .modify(0, |data_block: &mut DataBlock| {
                // 从源缓冲区定位本次读取的位置
                // buf[write_size..] 表示从已写入部分之后开始读取
                let src = &buf[write_size..write_size + block_write_size];
                // 计算块内偏移：start % BLOCK_SZ 得到在当前块内的起始位置
                // 从块内偏移开始，写入 block_write_size 字节
                let dest = &mut data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_write_size];
                // 将数据从源缓冲区复制到块缓存（会被标记为脏）
                dest.copy_from_slice(src);
            }); // modify 标记块为脏，延迟写回磁盘

            // 更新已写入字节数
            write_size += block_write_size;

            // ========== 检查是否完成 ==========
            // 如果当前块的结束位置就是整体写入的结束位置，写入完成
            if end_current_block == end {
                break;
            }

            // ========== 移动到下一个块 ==========
            // 移动到下一个块
            start_block += 1;
            // 更新当前写入位置为下一个块的起始位置
            start = end_current_block;
        }

        // 返回实际写入的字节数
        write_size
    }
}

impl DirEntry {
    pub fn init_empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_number: 0,
        }
    }

    pub fn new(name: &str, inode_number: u32) -> Self {
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_number,
        }
    }

    /// Serialize into bytes
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENT_SZ) }
    }
    /// Serialize into mutable bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENT_SZ) }
    }
    /// Get name of the entry
    pub fn name(&self) -> &str {
        let len = (0usize..).find(|i| self.name[*i] == 0).unwrap();
        core::str::from_utf8(&self.name[..len]).unwrap()
    }
    /// Get inode number of the entry
    pub fn inode_number(&self) -> u32 {
        self.inode_number
    }
}
