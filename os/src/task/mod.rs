use crate::loader::{get_num_app, get_app_data};
use crate::sbi::shutdown;
use crate::task::context::TaskContext;
use crate::task::switch::__switch;
use crate::trap::TrapContext;
use crate::{config::MAX_APP_NUM, sync::UPSafeCell};
use crate::task::task::{TaskControlBlock, TaskStatus};
use alloc::vec::Vec;
use lazy_static::*;
use log::{debug, info};

mod context;
mod switch;
mod task;

/// 任务管理器，负责管理所有的任务
/// 包括任务的创建、调度、切换等功能
/// 使用 UPSafeCell 包装以实现内部可变性
/// 任务管理器内部包含一个任务数量的变量和一个内部可变的任务管理器内部结构体
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

/// 任务管理器的内部结构体，包含所有任务的控制块和当前运行的任务 ID
/// 使用数组存储所有任务的控制块，大小为 MAX_APP_NUM
/// 当前运行的任务 ID 用于标识当前正在运行的任务
pub struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

lazy_static!{
    /// 全局唯一的任务管理器实例
    /// 
    /// 初始化时会加载所有用户应用程序，并将它们的状态设置为 Ready，同时预设好它们的上下文
    pub static ref TASK_MANAGER: TaskManager = {
        debug!("init task manager");
        let num_app = get_num_app();
        debug!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::with_capacity(MAX_APP_NUM);
         for i in 0..num_app {
            tasks.push(TaskControlBlock::new(
                get_app_data(i),
                i,
            ));
        }

        TaskManager {
            num_app,
            inner: unsafe { UPSafeCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            }) },
        }
    };
}

impl TaskManager {
    /// 运行第一个任务
    /// 
    /// 加载、运行第一个任务的过程为：
    /// 1. 获取任务管理器的内部可变引用
    /// 2. 将第一个任务的状态设置为 Running
    /// 3. 获取第一个任务的上下文指针
    /// 4. 释放任务管理器的内部可变引用
    /// 5. 创建一个未使用的上下文指针 `_unused_dummy_ctx_ptr`，并初始化为全零
    /// 6. 使用 `__switch` 函数切换到第一个任务的上下文
    /// 
    /// 返回值：
    /// - 该函数不会返回，因为它会切换到第一个任务的上下文，之后转入用户态执行
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];

        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);

        let mut _unused_dummy_ctx_ptr = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused_dummy_ctx_ptr as *mut TaskContext, next_task_cx_ptr);
        }

        unreachable!()
    }

    /// 将当前任务标记为挂起状态，通过修改任务管理器内部的任务的 `task_status` 字段实现
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].task_status = TaskStatus::Ready;
    }

    /// 将当前任务标记为退出状态，通过修改任务管理器内部的任务的 `task_status` 字段实现
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].task_status = TaskStatus::Exited;
    }

    /// 查找下一个可运行的任务
    /// 
    /// 在当前任务让出 CPU 后或者当前任务退出后，调用该函数查找下一个可运行的任务
    /// 查找过程为：
    /// 1. 获取任务管理器的内部可变引用
    /// 2. 从当前任务的下一个任务开始，循环查找状态为 Ready 的任务
    /// 3. 如果找到，则返回该任务的 ID
    /// 4. 如果没有找到，则返回 None
    /// 
    /// 返回值：
    /// - 如果找到下一个可运行的任务，返回 Some(任务 ID)
    /// - 如果没有找到可运行的任务，返回 None
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;

        (current_task + 1..current_task + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// 切换到下一个可运行的任务
    /// 
    /// 切换过程为：
    /// 1. 调用 `find_next_task` 查找下一个可运行的任务
    /// 2. 如果找到，则将当前任务的状态设置为 Ready，将下一个任务的状态设置为 Running
    /// 3. 获取当前任务和下一个任务的上下文指针
    /// 4. 释放任务管理器的内部可变引用
    /// 5. 使用 `__switch` 函数切换到下一个任务的上下文
    /// 6. 如果没有找到可运行的任务，则打印提示信息，并调用 `shutdown` 关闭系统
    /// 
    /// 注意：
    /// - 该函数假设至少有一个任务处于 Ready 状态，否则会调用 `shutdown` 关闭系统
    fn run_next_task(&self) {
        if let Some(next_task) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current_task = inner.current_task;
            inner.tasks[next_task].task_status = TaskStatus::Running;
            inner.current_task = next_task;

            let current_task_cx_ptr = &mut inner.tasks[current_task].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next_task].task_cx as *const TaskContext;
            drop(inner);

            // 调用 __switch 对于应用的上下文进行切换
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            info!("[kernel] All tasks are completed!");
            shutdown(false);
        }
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].get_user_token()
    }

    fn get_current_trap_cx(&self) -> &mut TrapContext {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].get_trap_cx()
    }

        /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }
}

/// 运行第一个任务的接口函数
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// 切换到下一个可运行任务的接口函数
pub fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// 将当前任务标记为挂起状态的接口函数
pub fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// 将当前任务标记为退出状态的接口函数
pub fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// 将当前任务挂起并切换到下一个任务的接口函数
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// 将当前任务退出并切换到下一个任务的接口函数
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
