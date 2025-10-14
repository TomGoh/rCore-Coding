use crate::sync::UPSafeCell;
use crate::task::task::TaskControlBlock;
use alloc::collections::vec_deque::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

/// 任务管理器，负责维护一个就绪的进程队列，
/// 该队列具体使用一个 `VecDeque` 双端队列实现，
/// 其中的每个任务由 `Arc<TaskControlBlock>` 智能指针管理
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    /// 创建一个新的任务管理器实例
    ///
    /// 返回值:
    /// - 返回一个 TaskManager 实例，其中包含一个空的就绪队列
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    /// 将一个任务添加到就绪队列的末尾
    ///
    /// 参数:
    /// - task: 要添加的任务，类型为 Arc<TaskControlBlock>
    /// 返回值:
    /// - 无返回值
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    /// 从就绪队列的前端取出一个任务
    ///
    /// 返回值:
    /// - 如果就绪队列非空，返回 Some(Arc<TaskControlBlock>)，否则返回 None
    /// - 该方法会从就绪队列中移除并返回队列前端的任务
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    /// 全局任务管理器实例，使用 UPSafeCell 包装以确保独占访问，
    /// 使用 lazy_static 进行延迟初始化
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = unsafe {
        UPSafeCell::new(TaskManager::new())
    };
}

/// 将一个任务添加到全局任务管理器的就绪队列末尾的函数接口
///
/// 参数:
/// - task: 要添加的任务，类型为 Arc<TaskControlBlock>
/// 返回值:
/// - 无返回值
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// 从全局任务管理器的就绪队列前端取出一个任务的函数接口
///
/// 返回值:
/// - 如果就绪队列非空，返回 Some(Arc<TaskControlBlock>)，否则返回 None
/// - 该函数会从就绪队列中移除并返回队列前
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
