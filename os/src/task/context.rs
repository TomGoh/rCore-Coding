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
    ///
    /// 返回值:
    /// - 返回一个 TaskContext 实例，其中所有寄存器均初始化为 0，
    ///   包括 ra、sp 和 s 寄存器数组
    pub fn zero_init() -> Self {
        TaskContext {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// 创建一个用于首次调度任务的 TaskContext
    ///
    /// 该函数用于初始化新创建任务的上下文，使得任务在首次被调度时能够正确跳转到
    /// `trap_return` 函数，从而完成从内核态到用户态的切换。
    ///
    /// 功能说明：
    /// - 将返回地址寄存器 `ra` 设置为 `trap_return` 函数的地址，这样当任务通过
    ///   `__switch` 函数被首次调度时，会跳转到 `trap_return` 执行
    /// - 将栈指针寄存器 `sp` 设置为内核栈顶地址，确保任务在内核态执行时有正确的栈空间
    /// - 将所有 callee-saved 寄存器 `s0-s11` 初始化为 0
    ///
    /// 使用场景：
    /// - 在 `TaskControlBlock::new` 中创建新进程时使用
    /// - 在 `TaskControlBlock::fork` 中创建子进程时使用
    ///
    /// 参数：
    /// - `kernel_stack_ptr`: 内核栈的栈顶地址，用于设置任务的栈指针
    ///
    /// 返回值：
    /// - 返回一个 TaskContext 实例，其 ra 指向 trap_return，sp 指向内核栈顶
    pub fn goto_trap_return(kernel_stack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kernel_stack_ptr,
            s: [0; 12],
        }
    }
}
