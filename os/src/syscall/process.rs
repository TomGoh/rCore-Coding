//! App management syscalls
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::{debug, info};

use crate::fs::{OpenFlags, open_file};
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::task::{
    MAX_SIG, SignalAction, SignalFlags, add_task, current_task, current_user_token,
    exit_current_and_run_next, pid2task, suspend_current_and_run_next,
};
use crate::timer::get_time_ms;

/// exit 的 System Call 实现
///
/// 参数:
/// - exit_code: 应用程序的退出码
///
/// 返回值:
/// - 该函数不会返回，调用后会切换到下一个应用程序
///
/// 注意:
/// - 该函数会打印应用程序的退出码
/// - 该函数假设当前有下一个应用程序可运行，当没有下一个应用程序运行时会关机
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {exit_code}");
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!"); // 这一行理论上不会被执行
}

/// yield 的 System Call 实现
///
/// 返回值:
/// - 成功时返回 0
///
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

    debug!("fork: new pid = {new_pid}");

    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    trap_cx.x[10] = 0; // 子进程 fork 返回值

    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);

    let mut args_vec = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        args = unsafe { args.add(1) };
    }

    info!("exec: path = {path}");

    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        info!("exec: file opened successfully, reading data");
        let all_data = app_inode.read_all();
        info!("exec: read {} bytes, calling task.exec", all_data.len());
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(), args_vec);
        info!("exec: task.exec completed, returning 0");
        argc as isize
    } else {
        info!("exec: failed to open file {path}");
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });

    if let Some((index, _)) = pair {
        let child = inner.children.remove(index);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}

pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if signum as usize > MAX_SIG {
        return -1;
    }

    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag, action as usize, old_action as usize) {
            return -1;
        }

        let prev_action = inner.signal_actions.table[signum as usize];
        *translated_refmut(token, old_action) = prev_action;
        inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
        0
    } else {
        -1
    }
}

pub fn sys_sigprocmask(mask: u32) -> isize {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        let old_mask_bits = inner.signal_mask.bits();
        if let Some(flag) = SignalFlags::from_bits(mask) {
            inner.signal_mask = flag;
            old_mask_bits as isize
        } else {
            -1
        }
    } else {
        -1
    }
}

fn check_sigaction_error(signal: SignalFlags, action: usize, old_action: usize) -> bool {
    action == 0
        || old_action == 0
        || signal.contains(SignalFlags::SIGKILL)
        || signal.contains(SignalFlags::SIGSTOP)
}

pub fn sys_kill(pid: usize, signum: i32) -> isize {
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signum) {
            let mut inner = task.inner_exclusive_access();
            // 检查信号是否已经存在
            if inner.pending_signals.contains(flag) {
                return -1;
            }
            inner.pending_signals.insert(flag);
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigreturn() -> isize {
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();

        inner.handling_sig = -1;

        // restore the trap context
        let trap_ctx = inner.get_trap_cx();
        *trap_ctx = inner.trap_cx_backup.unwrap();
        trap_ctx.x[10] as isize
    } else {
        -1
    }
}
