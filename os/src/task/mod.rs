use crate::loader::{get_num_app, init_app_context};
use crate::println;
use crate::sbi::shutdown;
use crate::task::context::TaskContext;
use crate::task::switch::__switch;
use crate::{config::MAX_APP_NUM, sync::UPSafeCell};
use crate::task::task::{TaskControlBlock, TaskStatus};
use lazy_static::*;
use log::debug;

mod context;
mod switch;
mod task;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

lazy_static!{
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];

        for (i, task) in tasks.iter_mut().enumerate().take(num_app) {
            let entry_point = init_app_context(i);
            task.task_cx = TaskContext::goto_restore(entry_point);
            task.task_status = TaskStatus::Ready;
            debug!("[kernel] App {} entry point = {:#x}", i, entry_point);
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner { tasks: tasks, current_task: 0 })
            }
        }
    };
}

impl TaskManager {
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

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;

        (current_task + 1..current_task + 1 + self.num_app)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) {
        if let Some(next_task) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current_task = inner.current_task;
            inner.tasks[next_task].task_status = TaskStatus::Running;
            inner.current_task = next_task;

            let current_task_cx_ptr = &mut inner.tasks[current_task].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next_task].task_cx as *const TaskContext;
            drop(inner);

            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            println!("[kernel] All tasks are completed! CPU halted.");
            shutdown(false);
        }
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

pub fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

pub fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}