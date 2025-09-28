use core::arch::global_asm;
use crate::task::context::TaskContext;

global_asm!(include_str!("switch.S"));

unsafe extern "C" {
    /// 在汇编代码中定义的任务切换函数，用于切换当前任务和下一个任务的上下文
    pub unsafe fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}