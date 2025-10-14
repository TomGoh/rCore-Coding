//! File and filesystem-related syscalls
use core::panic;

use crate::mm::page_table::translated_byte_buffer;
use crate::print;
use crate::sbi::console_getchar;
use crate::task::{current_user_token, suspend_current_and_run_next};
const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;
/// write 的 System Call 实现，本质上是对于 console::print 的封装
/// 目前仅支持向标准输出（fd=1）写入
///
/// 参数:
/// - fd: 文件描述符
/// - buf: 数据缓冲区指针
/// - len: 写入数据的长度
///
/// 返回值:
/// - 成功时返回写入的字节数
/// - 失败时触发 panic
///
/// 注意:
/// - 该函数假设 buf 指向的内存区域是有效且可读
/// - 仅支持 fd=1 (标准输出)，其他 fd 会触发 panic
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                let s = core::str::from_utf8(buffer).unwrap();
                print!("{s}");
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support reading 1 byte each time");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }

            let char_read = c as u8;
            let mut buffer = translated_byte_buffer(current_user_token(), buf, len);
            unsafe {
                buffer[0].as_mut_ptr().write_volatile(char_read);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
