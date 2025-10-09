/// rCore 的配置文件，主要包括内核中的栈起始位置，栈大小，用户 App 数量和大小等

pub const MAX_APP_NUM: usize = 16;
pub const USER_STACK_SIZE: usize = 4096 * 2; // 8KB
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 8KB

pub const KERNEL_HEAP_SIZE: usize = 0x30_0000; // 3MB
pub const PAGE_SIZE: usize = 0x1000; // 4KB
pub const PAGE_SIZE_BITS: usize = 0xc; // 12

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::board::{CLOCK_FREQ, MEMORY_END, MMIO};

/// 计算给定的程序对应的内核栈的位置范围，返回 (bottom, top)，
/// 主要是通过 TRAMPOLINE 和 KERNEL_STACK_SIZE 计算得到
/// 
/// 参数：
/// - `app_id`: App 的 ID，范围是 0 到 MAX_APP_NUM - 1
/// 
/// 返回值：
/// - `(usize, usize)`: 内核栈的底部和顶部地址
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}