//! App management syscalls
use crate::{println, task::{exit_current_and_run_next, suspend_current_and_run_next}};

/// exit 的 System Call 实现
/// 参数:
/// - exit_code: 应用程序的退出码
/// 返回值:
/// - 该函数不会返回，调用后会切换到下一个应用程序
/// 注意:
/// - 该函数会打印应用程序的退出码
/// - 该函数假设当前有下一个应用程序可运行，当没有下一个应用程序运行时会关机
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!"); // 这一行理论上不会被执行
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}
