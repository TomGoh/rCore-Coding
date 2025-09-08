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
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

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