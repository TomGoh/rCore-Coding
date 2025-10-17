mod context;

use core::{
    arch::{asm, global_asm},
    panic,
};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    println,
    syscall::syscall,
    task::{
        SignalFlags, check_signals_error_of_current, current_add_signal, current_trap_cx,
        current_user_token, exit_current_and_run_next, handle_signals,
        suspend_current_and_run_next,
    },
    timer::set_next_trigger,
};

// 汇编代码文件，定义了陷入处理程序的入口
global_asm!(include_str!("trap.S"));

/// 陷入机制的初始化函数
/// 该函数设置陷入处理程序的入口地址和模式
/// 在内核初始化阶段，将 stvec 设置为 trap_from_kernel
/// 这样如果在内核态发生陷入，会触发 panic
/// 注意:
/// - 该函数必须在内核初始化阶段调用一次
/// - 该函数使用了 unsafe 代码块，因为直接操作硬件寄存器
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[unsafe(no_mangle)]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();

    unsafe extern "C" {
        safe fn __alltraps();
        safe fn __restore();
    }

    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

/// 通用陷入处理函数
/// 该函数根据陷入的原因（由 scause 寄存器提供）
/// 进行不同的处理:
/// - 如果是用户态触发的系统调用，则调用 syscall 函数处理
///   并将结果存储在 x[0] 寄存器中，然后返回用户态
/// - 如果是存储错误或存储页面错误，则打印错误信息并杀死当前应用程序
/// - 如果是非法指令异常，则打印错误信息并杀死当前应用程序
/// - 对于其他未处理的异常，函数会 panic
///
/// 参数:
/// - cx: 当前的 TrapContext，上下文信息
///
/// 返回值:
/// - 返回修改后的 TrapContext，用于返回用户态
///
/// 注意:
/// - 该函数假设传入的 TrapContext 是有效的
/// - 该函数会修改 TrapContext 中的 sepc 和 x[0] 寄存器
/// - 该函数会调用 run_next_app 切换到下一个应用程序
/// - 该函数使用了 unsafe 代码块，因为直接操作硬件寄存器
#[unsafe(no_mangle)]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
            cx = current_trap_cx();
            cx.x[10] = result;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!(
                "[kernel] Page fault in application, bad addr = {:#x}, sepc = {:#x}",
                stval,
                current_trap_cx().sepc
            );
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!(
                "[kernel] Illegal instruction in application, sepc = {:#x}",
                current_trap_cx().sepc
            );
            current_add_signal(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}, sepc = {:#x}, sstatus = {:#x}",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
                current_trap_cx().sstatus.bits()
            );
        }
    }

    handle_signals();

    if let Some((errno, msg)) = check_signals_error_of_current() {
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }

    trap_return();
}

pub use context::TrapContext;
