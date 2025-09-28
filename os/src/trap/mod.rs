mod context;

use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};
use core::{arch::global_asm, panic};

use crate::{println, syscall::syscall};

// 汇编代码文件，定义了陷入处理程序的入口
global_asm!(include_str!("trap.S"));

/// 陷入机制的初始化函数
/// 该函数设置陷入处理程序的入口地址和模式
/// 具体来说，它将 stvec 寄存器设置为 __alltraps 函数的地址
/// 并将陷入模式设置为 TrapMode::Direct
/// 这样所有的陷入（异常和中断）都会跳转到 __alltraps 进行处理
/// 注意:
/// - 该函数必须在内核初始化阶段调用一次
/// - 该函数使用了 unsafe 代码块，因为直接操作硬件寄存器
pub fn init() {
    unsafe extern "C" { safe fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// 通用陷入处理函数
/// 该函数根据陷入的原因（由 scause 寄存器提供）
/// 进行不同的处理:
/// - 如果是用户态触发的系统调用，则调用 syscall 函数处理
///   并将结果存储在 x[0] 寄存器中，然后返回用户态
/// - 如果是存储错误或存储页面错误，则打印错误信息并杀死当前应用程序
/// - 如果是非法指令异常，则打印错误信息并杀死当前应用程序
/// - 对于其他未处理的异常，函数会 panic
/// 参数:
/// - cx: 当前的 TrapContext，上下文信息
/// 返回值:
/// - 返回修改后的 TrapContext，用于返回用户态
/// 注意:
/// - 该函数假设传入的 TrapContext 是有效的
/// - 该函数会修改 TrapContext 中的 sepc 和 x[0] 寄存器
/// - 该函数会调用 run_next_app 切换到下一个应用程序
/// - 该函数使用了 unsafe 代码块，因为直接操作硬件寄存器
#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        },
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] Page fault in application, bad addr = {:#x}, sepc = {:#x}", stval, cx.sepc);
            println!("[kernel] Killing application...");
            panic!("Page fault in application");
            // run_next_app();
        },
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] Illegal instruction in application, sepc = {:#x}", cx.sepc);
            println!("[kernel] Killing application...");
            panic!("Illegal instruction in application");
            // run_next_app();
        },
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}, sepc = {:#x}, sstatus = {:#x}",
                scause.cause(),
                stval,
                cx.sepc,
                cx.sstatus.bits()
            );
        },
    }
    cx
}

pub use context::TrapContext;