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

    /// 创建一个新的 TaskContext，用于从内核栈恢复任务
    /// 参数:
    /// - kernel_stack_ptr: 内核栈的栈顶指针
    /// 返回值:
    /// - 返回一个新的 TaskContext 实例，其中的 ra 寄存器指向 __restore 函数
    ///  sp 寄存器指向传入的 kernel_stack_ptr，s 寄存器数组初始化为全零
    /// 
    /// 注意:
    /// - 该函数使用了 extern "C" 声明的外部函数 __restore
    /// - 该函数将 __restore 的地址赋值给 ra 寄存器
    /// - 该函数将传入的 kernel_stack_ptr 赋值给 sp 寄存器
    /// - 该函数将 s 寄存器数组初始化为全零
    pub fn goto_restore(kernel_stack_ptr: usize) -> Self {
        unsafe extern "C" {
            unsafe fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kernel_stack_ptr,
            s: [0;12],
        }
    }
}