//! App management syscalls
use alloc::sync::Arc;
use log::{debug, info};

use crate::loader::get_app_data_by_name;
use crate::mm::page_table::{translated_refmut, translated_str};
use crate::timer::get_time_ms;
use crate::task::{add_task, current_task, current_user_token, exit_current_and_run_next, suspend_current_and_run_next};

/// exit 的 System Call 实现
/// 参数:
/// - exit_code: 应用程序的退出码
/// 返回值:
/// - 该函数不会返回，调用后会切换到下一个应用程序
/// 注意:
/// - 该函数会打印应用程序的退出码
/// - 该函数假设当前有下一个应用程序可运行，当没有下一个应用程序运行时会关机
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!"); // 这一行理论上不会被执行
}

/// yield 的 System Call 实现
/// 返回值:
/// - 成功时返回 0
/// 注意:
/// - 该函数会将当前任务挂起并切换到下一个任务
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().getpid() as isize
}

pub fn sys_fork() -> isize {
    let curr_task = current_task().unwrap();
    let new_task = curr_task.fork();
    let new_pid = new_task.getpid();

    debug!("fork: new pid = {}", new_pid);

    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    trap_cx.x[10] = 0; // 子进程 fork 返回值

    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);

    debug!("exec: path = {}", path);

    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if inner.children.iter().find(|p| { pid == -1 || pid as usize == p.getpid()})
    .is_none() {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });

    if let Some((index, _)) = pair {
        let child = inner.children.remove(index);
        assert_eq!(Arc::strong_count(&child), 1);
        let fount_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        fount_pid as isize
    } else {
        -2
    }
}