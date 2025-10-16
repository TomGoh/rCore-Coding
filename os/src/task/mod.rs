use crate::fs::OpenFlags;
use crate::fs::open_file;
use crate::task::task::TaskControlBlock;
use alloc::sync::Arc;
use lazy_static::*;

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

pub use context::TaskContext;
pub use manager::add_task;
#[allow(unused_imports)]
pub use pid::{KernelStack, PidAllocator, PidHandle, pid_alloc};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        #[cfg(feature = "test-mode")]
        let init_app_name = "initproc_test";
        #[cfg(not(feature = "test-mode"))]
        let init_app_name = "initproc";

        let inode = open_file(init_app_name, OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
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

/// 退出当前的任务进程，回收资源，处置子任务进程，调度其他就绪任务进程
///
/// 参数：
/// - exit_code： 当前任务进程执行后的返回退出值，设定在当前任务的 TCB 中
pub fn exit_current_and_run_next(exit_code: i32) {
    use crate::sbi::shutdown;
    use log::info;

    let curr_tcb = current_task().unwrap();

    // Check if current task is INITPROC
    let is_initproc = Arc::ptr_eq(&curr_tcb, &INITPROC);

    if is_initproc {
        // INITPROC is exiting - this means all tests are done
        info!("[kernel] INITPROC exiting with code {exit_code}");
        #[cfg(feature = "test-mode")]
        {
            if exit_code == 0 {
                info!("[kernel] All tests passed!");
                shutdown(false);
            } else {
                info!("[kernel] Tests failed with code {exit_code}");
                shutdown(true);
            }
        }
        #[cfg(not(feature = "test-mode"))]
        {
            info!("[kernel] System shutting down");
            shutdown(false);
        }
    }

    let mut inner = curr_tcb.inner_exclusive_access();
    // 标记该执行完成的进程为 Zombie，在 TCB 中记录退出值
    inner.task_status = task::TaskStatus::Zombie;
    inner.exit_code = exit_code;

    // 处置该进程的子进程，统一归类到 init 进程的子进程列表中，并更新对应子进程的父进程
    let mut init_proc_inner = INITPROC.inner_exclusive_access();
    for child in inner.children.iter() {
        child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
        init_proc_inner.children.push(child.clone());
    }
    drop(init_proc_inner);

    // 回收资源，包括：
    // - 清空子进程列表
    // - 回收数据页面
    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(curr_tcb);
    // 创建空的任务上下文以便切换到下一个就绪的任务进程
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}
