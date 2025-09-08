use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

/// 通过汇编代码发起系统调用的具体函数，使用 asm 宏嵌入 ecall 指令实现
/// asm 宏可以将汇编代码嵌入到局部的 Rust 实现的函数上下文中
/// id: 系统调用号
/// args: 系统调用的参数，最多支持 3 个参数
/// 返回值: 系统调用的返回值
/// 注意：该函数是一个低级接口，通常不直接调用，而是通过更高级的封装函数调用
fn syscall(id: usize, args :[usize; 3]) -> isize {
    let mut ret: isize;

    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

/// 根据RiscV的系统调用规范定义系统调用接口，
/// 本质是使用 Rust 对汇编的封装调用

/// 将内存中的数据写入到文件描述符 fd 指向的文件中
/// fd: 文件描述符
/// buf: 要写入的数据缓冲区
/// 返回值: 成功写入的字节数，失败返回负数错误码
/// syscall ID: 64
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buf.as_ptr() as usize, buf.len()])
}

/// 退出应用程序并将返回值 exit_code 返回给操作系统（当前是批处理系统）
/// exit_code: 退出码，通常为 0 表示成功，非 0 表示失败
/// 该函数不会返回
/// syscall ID: 93
pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}