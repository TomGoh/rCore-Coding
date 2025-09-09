//! File and filesystem-related syscalls
use crate::print;
const FD_STDOUT: usize = 1;

/// write 的 System Call 实现，本质上是对于 console::print 的封装
/// 目前仅支持向标准输出（fd=1）写入
/// 参数:
/// - fd: 文件描述符
/// - buf: 数据缓冲区指针
/// - len: 写入数据的长度
/// 返回值:
/// - 成功时返回写入的字节数
/// - 失败时触发 panic
/// 注意:
/// - 该函数假设 buf 指向的内存区域是有效且可读
/// - 仅支持 fd=1 (标准输出)，其他 fd 会触发 panic
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
