use crate::loader::get_app_data_by_name;
use crate::task::task::TaskControlBlock;
use lazy_static::*;
use alloc::sync::Arc;

mod context;
mod switch;
mod task;
mod pid;
mod manager;
mod processor;

pub use context::TaskContext;
pub use manager::add_task;
pub use pid::{KernelStack, PidAllocator, PidHandle, pid_alloc};
pub use processor::{
    Processor, current_task, current_trap_cx, current_user_token, run_tasks, schedule,
    take_current_task,
};

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(
        TaskControlBlock::new(get_app_data_by_name("initproc").unwrap())
    );
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// 挂起当前任务并运行下一个任务
/// 
/// 主要是通过：
/// - 获取当前任务并将其状态设置为就绪
/// - 获取当前任务的上下文指针
/// - 将当前任务重新添加到就绪队列
/// - 使用当前任务的上下文指针作为参数调用调度函数切换到下一个任务，
///   当前任务的上下文指针地址会被 schedule 函数调用 __switch 汇编函数
///   隐式地设置在 Processor 结构体的 idle_task_cx 字段中
pub fn suspend_current_and_run_next() {
    let curr_task = take_current_task().unwrap();
    let mut curr_task_inner = curr_task.inner_exclusive_access();

    let curr_task_cxptr = &mut curr_task_inner.task_cx as *mut TaskContext;
    curr_task_inner.task_status = task::TaskStatus::Ready;
    drop(curr_task_inner);

    add_task(curr_task);
    schedule(curr_task_cxptr);
} 