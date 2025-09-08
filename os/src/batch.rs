use crate::{ println, sync::UPSafeCell };
use lazy_static::*;
use crate::sbi::*;
use core::arch::asm;

const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x8040_0000;
const APP_SIZE_LIMIT: usize = 0x0020_0000; // 2MB

/// 管理应用程序的结构体
/// 包含应用程序的数量、当前运行的应用程序索引
/// 以及每个应用程序的起始地址数组
/// 注意 app_start 数组的长度为 MAX_APP_NUM + 1
/// 以便存储最后一个应用程序的结束地址
struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM+1],
}

impl AppManager {
    /// 打印应用程序的信息
    /// 包括应用程序的数量和每个应用程序的起始及结束地址
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!("[kernel] app[{}]: {:#x} ~ {:#x}\n", 
                i, 
                self.app_start[i], 
                self.app_start[i+1]
            );
        }
    }

    /// 加载指定 ID 的应用程序到内存的预定义位置
    /// 如果 app_id 超过应用程序数量，则打印完成信息并关闭系统
    /// 否则将应用程序代码复制到 APP_BASE_ADDRESS 处
    /// 并执行指令缓存同步指令以确保新代码被正确执行
    /// # Panics
    /// 如果 app_id 超过应用程序数量，则系统将关闭，不会返回
    pub fn load_app(&self, app_id: usize) {
        // 如果 app_id 超过应用程序数量，打印信息并关闭系统
        if app_id >= self.num_app {
            println!("All applications completed!");
            shutdown(false);
        }

        println!("[kernel] Loading app {}", app_id);
        unsafe {
            // 清空应用程序加载区域，该区域起始位置为预定义的 APP_BASE_ADDRESS
            // 大小为 APP_SIZE_LIMIT
            core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
            // 获得当前指定任务的起始和结束地址，并将这部分代码数据首先复制到 app_src 切片
            let app_src = core::slice::from_raw_parts(
                self.app_start[app_id] as *const u8,
                self.app_start[app_id + 1] - self.app_start[app_id]
            );
            // 定义目标内存区域的切片（引用） app_dst
            // 该区域从 APP_BASE_ADDRESS 开始，大小与 app_src 相同
            let app_dst = core::slice::from_raw_parts_mut(
                APP_BASE_ADDRESS as *mut u8,
                app_src.len()
            );
            // 将 app_src 的内容复制到 app_dst，实现应用程序的加载
            app_dst.copy_from_slice(app_src);
            // 执行指令缓存同步指令，确保新加载的代码能够被正确执行
            // 这是因为某些处理器架构可能会缓存指令
            // 需要通过该指令来刷新缓存
            asm!("fence.i");
        }
    }

    /// 获取当前运行的应用程序索引
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    /// 切换到下一个应用程序
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {

    // 使用 lazy_static 宏在运行时初始化全局变量 APP_MANAGER
    // 仅仅在第一次访问时进行初始化
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            // 首先找到 linker_app.S 中定义的 num_app 变量
            // 从这个符号开始解析 app_start 数组
            unsafe extern "C" {
                safe fn _num_app();
            }

            // 获取 _num_app 的地址，并读取其值
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();
            // 根据读取的 num_app 值，读取 app_start 数组， 确保不超过 MAX_APP_NUM
            assert!(num_app <= MAX_APP_NUM);
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM+1];
            // 从 num_app_ptr 的下一个地址开始，读取 num_app + 1 个 usize 值，
            // 分别对应每个应用程序的起始地址和最后一个应用程序的结束地址
            let app_start_raw: &[usize] = core::slice::from_raw_parts(
                num_app_ptr.add(1),
                num_app + 1
            );
            // 将读取到的地址复制到 app_start 数组中
            app_start[..=num_app].copy_from_slice(app_start_raw);
            // 根据读取到的任务数量和任务的起始地址初始化 AppManager 结构体
            AppManager {
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}