use riscv::register::sstatus::{self, SPP, Sstatus};

/// 保存陷入内核态时用户态程序运行的上下文
#[repr(C)]
pub struct TrapContext {
    // 保存 x0 ~ x31 寄存器的数组
    pub x: [usize; 32],
    // 保存陷入内核态时的 sstatus 寄存器
    pub sstatus: Sstatus,
    // 保存陷入内核态时的 sepc 寄存器，即用户态程序的下一条指令地址
    // 用于返回用户态程序时设置到 sepc 寄存器
    // 防止嵌套 Trap 时丢失用户态程序的返回地址
    pub sepc: usize,
}

impl TrapContext { 
    /// 设置用户栈指针，将 x2 寄存器设置为指定的 sp 值
    /// 参数:
    /// - sp: 用户栈顶地址
    /// 注意:
    /// - 该函数仅修改 x2 寄存器的值
    /// - 该函数不会检查 sp 的有效性
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// 创建一个新的 TrapContext，用于初始化用户态程序的上下文
     /// 参数:
     /// - entry: 用户态程序的入口地址
     /// - sp: 用户态程序的栈顶地址
     /// 返回值:
     /// - 返回一个初始化好的 TrapContext 实例
     /// 注意:
     /// - 该函数会将 sstatus 寄存器的 SPP 位设置为 User，表示下次从内核态返回时进入用户态
     /// - sepc 寄存器会被设置为 entry，表示用户态程序从该地址开始执行
     /// - x2 寄存器（栈指针）会被设置为 sp，表示用户态程序的栈顶位置
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }
}