//! App management syscalls
use log::{debug, info};

use crate::loader::get_app_data_by_name;
use crate::mm::page_table::translated_str;
use crate::timer::get_time_ms;
use crate::task::{add_task, current_task, current_user_token, suspend_current_and_run_next};

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
    exit_current_and_run_next();
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