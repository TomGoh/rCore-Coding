//! File and filesystem-related syscalls
use alloc::sync::Arc;

use crate::fs::{OpenFlags, make_pipe, open_file};
use crate::mm::{UserBuffer, translated_byte_buffer, translated_refmut, translated_str};
use crate::task::{current_task, current_user_token};

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
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);

    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd > inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();

    let (pipe_read_end, pipe_write_end) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read_end);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write_end);
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }

    if inner.fd_table[fd].is_none() {
        return -1;
    }

    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
}
