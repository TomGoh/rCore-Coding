/// rCore 的配置文件，主要包括内核中的栈起始位置，栈大小，用户 App 数量和大小等

pub const MAX_APP_NUM: usize = 16;
pub const APP_BASE_ADDRESS: usize = 0x8040_0000;
pub const APP_SIZE_LIMIT: usize = 0x0002_0000; // 128KB
pub const USER_STACK_SIZE: usize = 4096 * 2; // 8KB
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 8KB

pub const KERNEL_HEAP_SIZE: usize = 0x30_0000; // 3MB

pub use crate::board::CLOCK_FREQ;