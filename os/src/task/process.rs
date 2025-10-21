use super::TaskControlBlock;
use super::manager::insert_into_pid2process;
use super::{PidHandle, pid_alloc};
use super::{SignalFlags, add_task};
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{KERNEL_SPACE, MemorySet, translated_refmut};
use crate::sync::{Condvar, Mutex, Semaphore, UPSafeCell};
use crate::trap::{TrapContext, trap_handler};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;
use ida_rs::Ida;

/// 进程控制块，包含进程的各种信息和状态，
/// 实际状态主要存储在内部的 inner 字段中
pub struct ProcessControlBlock {
    pub pid: PidHandle,
    inner: UPSafeCell<ProcessControlBlockInner>,
}

/// 实际存储进程状态和信息的结构体
pub struct ProcessControlBlockInner {
    pub is_zombie: bool,
    /// 应用的内存地址空间
    pub memory_set: MemorySet,
    /// 父进程的弱引用，可以为 None
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// 子进程的强引用列表
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// 退出码，仅当状态为 Zombie 时有效
    pub exit_code: i32,
    /// 文件描述符表
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    /// 信号屏蔽字
    pub signal_mask: SignalFlags,
    /// 待处理的信号集合
    pub pending_signals: SignalFlags,
    /// 任务是否被标记为已终止
    pub killed: bool,
    /// 任务是否被标记为已冻结
    pub frozen: bool,
    /// 目前正在被处理的信号例程
    pub handling_sig: isize,
    /// 当前进程下的所有线程（也就是Task）
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    /// 进程的资源分配器，主要用于分配 tid
    pub task_res_allocator: Ida,
    /// 当前进程拥有的 Mutex 的列表
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    /// 当前进程拥有的 Semaphore 的列表
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    /// 当前进程拥有的 CondVar 的列表
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
}

impl ProcessControlBlockInner {
    /// 获取任务对应的页表的页号
    ///
    /// 返回值:
    /// - `usize`：返回任务对应的页表的页号
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc().unwrap()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.free(tid)
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}

impl ProcessControlBlock {
    /// 获取任务控制块内部的独占可变引用
    ///
    /// 返回值:
    /// - `RefMut<'_, TaskControlBlockInner>`：返回任务控制块内部的独占可变引用
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }

    /// 依据 ELF 文件数据创建一个新的进程控制块，以智能指针 `Arc<ProcessControlBlock` 的形式返回
    ///
    /// 参数：
    /// - elf_data: ELF 文件的字节切片引用
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // 首先，调用 `MemorySet::from_elf` 函数从 ELF 文件数据中创建内存映射，
        // 并获取映射完成后的用户栈顶地址和程序入口点以及内存映射对象
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        let pid_handle = pid_alloc();

        // 然后，根据上述初始化数据创建内部的任务控制块结构体 `ProcessControlBlockInner`，
        // 并将其包装在 `UPSafeCell` 中以确保独占访问
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    pending_signals: SignalFlags::empty(),
                    signal_mask: SignalFlags::empty(),
                    handling_sig: -1,
                    killed: false,
                    frozen: false,
                    tasks: Vec::new(),
                    task_res_allocator: Ida::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                })
            },
        });

        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));

        // 最后，读取刚刚初始化的当前应用对应的 TCB 中的 TrapContext，
        // 并调用 `TrapContext::app_init_context` 方法设置其初始上下文，
        // 包括入口点、用户栈顶地址、内核栈顶地址以及陷阱处理函数地址
        // 以便应用程序在首次运行时能够正确跳转到用户态执行
        // 并返回新创建的任务控制块实例
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );

        let mut process_inner = process.inner.exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        add_task(task);
        process
    }

    /// 获取任务的 PID
    ///
    /// 返回值:
    /// - `usize`：返回任务的 PID
    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    /// 使用新的 ELF 文件数据替换当前任务的内存空间和上下文
    ///
    /// 参数:
    /// - elf_data: ELF 文件的字节切片引用
    /// - args: 命令行参数的字符串向量
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>) {
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // 首先，从 ELF 文件数据中创建新的内存映射，
        // 并获取用户栈顶地址和程序入口点
        // 注意，由于物理内存的 FrameTracker 实现了 Drop trait，
        // 因此当旧的 MemorySet 被新的替换时，旧的内存映射
        // 会自动释放其占用的物理内存
        // 这里不需要手动释放旧的内存映射
        // 只需要创建新的内存映射并替换即可
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();
        self.inner_exclusive_access().memory_set = memory_set;

        // 而后，利用创建的内存映射与陷入上下文信息
        // 更新当前任务控制块内部的内存映射对象和 TrapContext，
        // 并重新设置 TrapContext 以反映新的内存空间和程序入口
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_base = user_sp;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();

        // 接着，需要将命令行参数写入到新的用户栈中
        // 以便于新程序可以通过栈参数获取命令行参数，也就是分配一个字符串指针数组 argv
        // 首先，计算参数字符串和指针所需的空间，通过调整用户栈顶地址
        // 为参数字符串和指针预留空间
        // 预留的空间大小为所有参数字符串的长度之和加上每个字符串的结尾空字符
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        // 然后，创建参数指针数组，并将每个参数字符串写入到用户栈中
        // 同时将每个参数字符串的地址存储在参数指针数组中
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    new_token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;

        // 接着，将每个参数字符串的地址存储在参数指针数组中
        // 并将参数字符串写入到用户栈中
        // 通过逐个参数、逐个字符写入的方式，首先调整用户栈顶地址以为参数字符串预留空间
        // 然后，将参数字符串的每个字符写入到用户栈中
        // 实际写入的时候需要使用 translated_refmut 函数
        // 以确保正确的地址转换和内存访问权限
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(new_token, p as *mut u8) = *c;
                p += 1;
            }
            // 手动添加字符串结尾的空字符 '\0'
            *translated_refmut(new_token, p as *mut u8) = 0;
        }
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // 使用新的入口点、用户栈顶地址和参数信息重新设置 TrapContext
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len(); // 将 argc 传递给用户程序
        trap_cx.x[11] = argv_base; // 将 argv 传递给用户程序
        *task_inner.get_trap_cx() = trap_cx;
        // 在 exec 中无需对于任务上下文进行额外处理
        // 因为当前任务本身已经在执行了，
        // 只需要更新内存映射和 TrapContext 即可，
        // 只有在运行中暂停的任务才需要保存和恢复其上下文至内核栈中
    }

    /// 创建当前任务的一个子任务，主要是通过复制当前任务的内存空间和状态来实现
    ///
    /// 参数：
    /// - `self: &Arc<ProcessControlBlock>`：当前进程的引用计数智能指针
    ///
    /// 返回值：
    /// - `Arc<ProcessControlBlock>`：返回新创建的子进程的引用计数智能指针
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // 首先，获取当前任务的内部可变引用，以便访问和修改其状态
        let mut parent_inner = self.inner_exclusive_access();
        assert_eq!(parent_inner.thread_count(), 1);
        // 然后，调用 `MemorySet::from_existing_user` 方法复制当前任务的内存空间
        let memory_set = MemorySet::from_existing_user(&parent_inner.memory_set);
        // 分配一个新的 PID 句柄
        let pid_handle = pid_alloc();

        // 接下来，复制当前任务的文件描述符表
        // 这里简单地通过克隆每个文件描述符的引用计数智能指针来实现
        // 以便于子任务可以共享父任务打开的文件
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }

        // 接着，创建一个新的任务控制块实例 `ProcessControlBlock` 作为子任务，
        // 基于前述的初始化工作，同时设置其父任务为当前任务的弱引用，更新其状态为 Ready
        let child_pcb = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    pending_signals: SignalFlags::empty(),
                    signal_mask: parent_inner.signal_mask,
                    handling_sig: -1,
                    killed: false,
                    frozen: false,
                    task_res_allocator: Ida::new(),
                    tasks: Vec::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                })
            },
        });

        // 将刚刚创建的子任务添加到所对应的父任务的子任务列表中
        // 以便于父任务可以管理和等待其子任务
        parent_inner.children.push(child_pcb.clone());

        // 为子进程创建主线程
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child_pcb),
            parent_inner
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));

        let mut child_inner = child_pcb.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);

        // 最后，读取新创建的子任务对应的 TCB 中的 TrapContext，
        // 重新设置 TrapContext，这是因为子任务的内存空间是复制自父任务的，
        // 此时的 TrapContext 中的内核栈顶地址仍然是父任务的，由于内核栈因为
        // PID 不同而不同，因此需要重新设置该函数中新创建的子任务的内核栈顶地址
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();

        drop(task_inner);
        insert_into_pid2process(child_pcb.getpid(), Arc::clone(&child_pcb));
        // 添加当前进程的主线程到任务调度器中
        add_task(task);

        child_pcb
    }
}
