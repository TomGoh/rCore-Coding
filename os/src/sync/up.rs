use core::cell::{RefCell, RefMut};
/// 对于 RefCell 的封装
/// 确保对于内部数据的独占访问：
/// 每次使用的时候需要使用 exclusive_access 方法获取独占访问权限
pub struct UPSafeCell<T> {
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl <T> UPSafeCell<T> {
    /// 创建一个新的 UPSafeCell
    /// # Safety
    /// 该函数不保证数据竞争的安全性
    /// 需要调用者保证仅仅在单核上使用
    pub unsafe fn new(inner: T) -> Self {
        Self {
            inner: RefCell::new(inner),
        }
    }

    /// 任何对于内部数据的访问都必须通过该方法获取独占访问权限
    /// 该方法返回一个 RefMut 智能指针，确保在其生命周期
    /// 内对数据的独占访问
    /// # Panics
    /// 如果在调用该方法时已经存在对数据的不可变引用
    /// 则该方法会 panic
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}