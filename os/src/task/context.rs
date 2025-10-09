use crate::trap::trap_return;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    /// 创建一个全零初始化的 TaskContext
    /// 返回值:
    /// - 返回一个 TaskContext 实例，其中所有寄存器均初始化为 0，
    /// 包括 ra、sp 和 s 寄存器数组
    pub fn zero_init() -> Self {
        TaskContext {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub fn goto_trap_return(kernel_stack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kernel_stack_ptr,
            s: [0;12],
        }
    }
}