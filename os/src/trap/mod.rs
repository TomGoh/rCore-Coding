mod context;

use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};
use core::{arch::global_asm, panic};

use crate::{println, syscall::syscall};
use crate::batch::run_next_app;

global_asm!(include_str!("trap.S"));

pub fn init() {
    unsafe extern "C" { safe fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[0] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        },
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] Page fault in application, bad addr = {:#x}, sepc = {:#x}", stval, cx.sepc);
            println!("[kernel] Killing application...");
            run_next_app();
        },
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] Illegal instruction in application, sepc = {:#x}", cx.sepc);
            println!("[kernel] Killing application...");
            run_next_app();
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