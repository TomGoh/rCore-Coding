#![no_std]
#![feature(linkage)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;


#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main()); // 此处的 main 是用户在 bin 中的用户程序定义的 main 函数
    panic!("unreachable after exit");
}

fn clear_bss() {
    unsafe extern "C"{
        safe fn start_bss();
        safe fn end_bss();
    }
    (start_bss as usize..end_bss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

/// 定义一个弱符号的 main 函数，防止用户没有定义 main 函数时链接失败
/// 如果用户定义了 main 函数，则会覆盖这个弱符号
/// 如果用户没有定义 main 函数，则会调用这个弱符号
/// 编译可以通过，但是在运行时会 panic
/// 这样可以提醒用户必须定义 main 函数
#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("no main function found");
}

use syscall::*;

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code);
}