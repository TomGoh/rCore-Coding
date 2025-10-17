use crate::task::{MAX_SIG, SignalFlags};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
/// 注意：此处将 SignalAction 指定为 repr(C, align(16))，一方面可满足
/// RISC-V 在用户态与内核态之间传送结构体时的对齐要求，避免访问异常；
/// 另一方面，结构体大小为 16 字节且 16 字节对齐意味着它只会在页内的
/// 0x0、0x10、…、0xFF0 等偏移处开头，从而确保单个 SignalAction 不会跨越
/// 4 KiB 页边界，减少跨页访问带来的处理复杂度与潜在风险。
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

#[derive(Clone)]
pub struct SignalActions {
    pub table: [SignalAction; MAX_SIG + 1],
}

impl Default for SignalAction {
    fn default() -> Self {
        SignalAction {
            handler: 0,
            mask: SignalFlags::from_bits(40).unwrap(),
        }
    }
}

impl Default for SignalActions {
    fn default() -> Self {
        Self {
            table: [SignalAction::default(); MAX_SIG + 1],
        }
    }
}
