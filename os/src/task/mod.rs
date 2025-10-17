#![allow(unused)]
mod action;
mod context;
mod manager;
mod pid;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::fs::{OpenFlags, open_file};
use crate::println;
use crate::sbi::shutdown;
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
use manager::fetch_task;
use manager::remove_from_pid2task;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use action::{SignalAction, SignalActions};
pub use manager::{add_task, pid2task};
pub use pid::{KernelStack, PidHandle, pid_alloc};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
pub use signal::{MAX_SIG, SignalFlags};

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
    remove_from_pid2task(curr_tcb.getpid());

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

pub fn current_add_signal(signal: SignalFlags) {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        inner.pending_signals |= signal;
    }
}

pub fn handle_signals() {
    loop {
        check_pending_signals();
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let inner = task.inner_exclusive_access();
            (inner.frozen, inner.killed)
        };
        if !frozen || killed {
            break;
        }
        suspend_current_and_run_next();
    }
}

fn check_pending_signals() {
    for sig in 0..(MAX_SIG + 1) {
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();

        if inner.pending_signals.contains(signal) && (!inner.signal_mask.contains(signal)) {
            let mut masked = true;
            let handling_sig = inner.handling_sig;
            if handling_sig == -1 {
                masked = false;
            } else {
                let handling_sig = handling_sig as usize;
                if !inner.signal_actions.table[handling_sig]
                    .mask
                    .contains(signal)
                {
                    masked = false;
                }
            }

            if !masked {
                drop(inner);
                drop(task);

                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    call_kernel_signal_handler(signal);
                } else {
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}

fn call_kernel_signal_handler(sig: SignalFlags) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    match sig {
        SignalFlags::SIGSTOP => {
            inner.frozen = true;
            inner.pending_signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if inner.pending_signals.contains(SignalFlags::SIGCONT) {
                inner.pending_signals ^= SignalFlags::SIGCONT;
                inner.frozen = false;
            }
        }
        _ => {
            inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    let handler = inner.signal_actions.table[sig].handler;
    if handler != 0 {
        inner.handling_sig = sig as isize;
        inner.pending_signals ^= signal;

        let old_trap_cx = inner.get_trap_cx();
        inner.trap_cx_backup = Some(*old_trap_cx);

        old_trap_cx.sepc = handler;
        old_trap_cx.x[10] = sig;
    } else {
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    inner.pending_signals.check_error()
}
