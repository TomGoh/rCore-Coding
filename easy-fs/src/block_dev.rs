use core::any::Any;

/// 块设备接口，定义了块设备的读写操作的方法
/// 要求实现该接口的类型必须是线程安全的（`Send + Sync`）并且支持运行时类型识别（`Any`）
/// 注意：读写的单位是块（block），每个块的大小由具体的块设备实现决定
pub trait BlockDevice: Send + Sync + Any {
    /// 读取指定块的数据到缓冲区
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    /// 将缓冲区的数据写入指定块
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
