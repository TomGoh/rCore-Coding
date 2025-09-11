use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

/// 每个应用程序的内核栈
/// 该栈在内存中对齐到 4096 字节边界
/// 大小为 KERNEL_STACK_SIZE 字节
#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

/// 每个应用程序的用户栈
/// 该栈在内存中对齐到 4096 字节边界
/// 大小为 USER_STACK_SIZE 字节
#[repr(align(4096))]
#[derive(Copy, Clone)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

/// 内核栈，每个应用程序有一个对应的内核栈
/// 使用静态数组存储，大小为 MAX_APP_NUM
/// 每个内核栈的大小为 KERNEL_STACK_SIZE
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

/// 用户栈，每个应用程序有一个对应的用户栈
/// 使用静态数组存储，大小为 MAX_APP_NUM
/// 每个用户栈的大小为 USER_STACK_SIZE
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl UserStack {
    /// 获取用户栈的栈顶指针
    /// 返回值:
    /// - 返回用户栈的栈顶指针（即用户栈的最高地址）
    /// 注意:
    /// - 该函数假设用户栈是向下增长的，因此栈顶指针是栈底地址加上栈大小
    /// - 该函数使用了 as_ptr() 方法获取栈底地址，并将其转换为 usize 类型
    /// - 该函数返回的栈顶指针是一个 usize 类型的整数
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

impl KernelStack {
    /// 获取内核栈的栈顶指针
    /// 返回值:
    /// - 返回内核栈的栈顶指针（即内核栈的最高地址）
    /// 注意:
    /// - 该函数假设内核栈是向下增长的，因此栈顶指针是栈底地址加上栈大小
    /// - 该函数使用了 as_ptr() 方法获取栈底地址，并将其转换为 usize 类型
    /// - 该函数返回的栈顶指针是一个 usize 类型的整数
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    /// 将一个 TrapContext 压入内核栈
    /// 参数:
    /// - context: 需要压入的 TrapContext 实例
    /// 返回值:
    /// - 返回新的栈顶指针（即 TrapContext 在内核栈中的地址）
    /// 注意:
    /// - 该函数假设内核栈有足够的空间来存放新的 TrapContext
    /// - 该函数使用了 unsafe 代码块，因为直接操作内存指针
    pub fn push_context(&self, context: TrapContext) -> usize {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = context;
        }
        cx_ptr as usize
    }
}

/// 根据应用程序 ID 获取应用程序的基地址
/// 参数:
/// - app_id: 应用程序的 ID（从 0 开始）
/// 返回值:
/// - 返回应用程序的基地址
/// 注意:
/// - 该函数假设应用程序的基地址是从 APP_BASE_ADDRESS 开始
///  并且每个应用程序占用 APP_SIZE_LIMIT 字节的空间
fn get_base_addr_by_app_id(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// 获取应用程序的数量
/// 返回值:
/// - 返回应用程序的数量
/// 注意:
/// - 该函数通过读取链接脚本中定义的 _num_app 符号，通过读取该符号的值来获取应用程序的数量
/// - 该函数使用了 unsafe 代码块，因为直接操作内存指针
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        safe fn _num_app();
    }

    unsafe{ (_num_app as usize as *const usize).read_volatile() }
}

/// 清楚应用程序的内存空间，并将应用程序从链接脚本中加载到内存中
/// 
/// 主要的实现方法为：
/// - 通过 _num_app 符号获取应用程序的数量
/// - 通过 _num_app 符号获取每个应用程序在链接脚本中的起始地址以及结束地址
/// - 针对每个应用程序的内存区域，首先将其全部清零
/// - 然后将链接脚本中的应用程序代码复制到对应的内存区域
/// - 最后使用 fence.i 指令刷新指令缓存，确保 CPU 能够正确地执行新加载的代码
pub fn load_apps() {
    unsafe extern "C" {
        safe fn _num_app();
    }

    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe {
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app+1)
    };

    for i in 0..num_app {
        let base_i = get_base_addr_by_app_id(i);
        (base_i..base_i + APP_SIZE_LIMIT).for_each(|address| {
            unsafe {
                (address as *mut u8).write_volatile(0);            
            }
        });
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i+1] - app_start[i])
        };
        let dst = unsafe {
            core::slice::from_raw_parts_mut(base_i as *mut u8, src.len())
        };
        dst.copy_from_slice(src);
    }

    unsafe {
        asm!("fence.i");
    }
}

pub fn init_app_context(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_addr_by_app_id(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}