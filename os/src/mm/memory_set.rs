use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use log::debug;
use crate::{config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE}, mm::{address::{PhysPageNum, StepByOne, VPNRange, VirtAddr, VirtPageNum}, frame_allocator::{frame_alloc, FrameTracker}, page_table::{PTEFlags, PageTable}}};

// 定义了一些外部符号，这些符号通常是在链接阶段由链接器脚本定义的，
// 用于标识内核映像中的特定段的起始和结束地址
unsafe extern "C" {
    safe fn stext();
    safe fn etext();
    safe fn srodata();
    safe fn erodata();
    safe fn sdata();
    safe fn edata();
    safe fn sbss_with_stack();
    safe fn ebss();
    safe fn ekernel();
    safe fn strampoline();
}

/// 映射类型，表示逻辑段的映射方式
/// 在 rCore 中定义并实现了两种映射类型：
/// - `Identical`： 该类型表示虚拟地址与物理地址相同，
///    即虚拟地址空间中的某个地址直接映射到物理地址空间中的相同地址
/// - `Framed`： 该类型表示虚拟地址与物理地址不同，
///    即虚拟地址空间中的某个地址映射到物理地址空间中的某个页框
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    /// 映射权限，表示逻辑段的访问权限
    /// 使用 `bitflags` 宏定义了四种权限标志：
    /// - `R`： 读权限，表示该逻辑段可以被读取
    /// - `W`： 写权限，表示该逻辑段可以被写入
    /// - `X`： 执行权限，表示该逻辑段可以被执行
    /// - `U`： 用户权限，表示该逻辑段可以被用户态程序访问
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

/// 逻辑段，代表一段连续地址的物理内存，
/// 具有四个成员变量：
/// - `vpn_range`： `VPNRange`，表示该逻辑段所包含的虚拟页号范围，
///    从 l 到 r（不包括 r），可以通过迭代器访问每个虚拟页号
/// - `data_frames`： `BTreeMap<VirtPageNum, FrameTracker>`，表示该逻辑段中所有地址从虚拟页号到物理页框的映射
/// - `map_type`： `MapType`，表示该逻辑段的映射类型，可以是 `Identical` 或 `Framed`
/// - `map_permission`： `MapPermission`，表示该逻辑段的权限，可以是读、写、执行和用户权限的组合
pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_permission: MapPermission,
}

impl MapArea {
    /// 创建一个新的逻辑段，主要是通过传入的起始和结束虚拟地址来确定逻辑段所包含的虚拟页号范围,
    /// 同时还需要指定映射类型和权限
    /// 
    /// 参数：
    /// - `start_va`： 逻辑段的起始虚拟地址
    /// - `end_va`： 逻辑段的结束虚拟地址
    /// - `map_type`： 逻辑段的映射类型，可以是 `Identical` 或 `Framed`
    /// - `map_permission`： 逻辑段的权限，可以是读、写、执行和用户权限的组合
    /// 
    /// 返回值：
    /// - `Self`： 返回一个新的 `MapArea` 实例，表示创建的逻辑段
    pub fn new(start_va: VirtAddr, end_va: VirtAddr, map_type: MapType, map_permission: MapPermission) ->Self{
        let start_vpn = start_va.floor();
        let end_vpn = end_va.ceil();

        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_permission,
        }
    }

    /// 映射一个虚拟页号到物理页框，
    /// 具体的实现是：
    /// 1. 根据映射类型分配物理页框，
    ///    如果是 `Framed` 类型，则调用 `frame_alloc` 分配一个新的物理页框，并将其记录在 `data_frames` 中；
    ///    如果是 `Identical` 类型，则直接将虚拟页号转换为物理页号
    /// 2. 根据映射权限创建页表项标志
    /// 3. 调用页表的 `map` 方法完成从虚拟页号到物理页框的映射
    /// 
    /// 参数：
    /// - `page_table`： 页表，用于完成映射操作
    /// - `vpn`：需要被映射的虚拟页号
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_permission.bits() as usize).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    /// 取消映射一个虚拟页号，
    /// 具体的实现是：
    /// 1. 根据映射类型进行不同的处理，
    ///    如果是 `Framed` 类型，则需要从 `data_frames` 中移除对应的物理页框；
    ///    如果是 `Identical` 类型，则不需要额外进行任何操作
    /// 2. 调用页表的 `unmap` 方法完成取消映射操作
    /// 
    /// 参数：
    /// - `page_table`： 页表，用于完成取消映射操作
    /// - `vpn`： 需要被取消映射的虚拟页号
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }

    /// 将整个逻辑段映射到页表中，
    /// 具体的实现是遍历逻辑段所包含的所有虚拟页号，
    /// 并调用 `map_one` 方法将每个虚拟页号映射到页表中
    /// 
    /// 参数：
    /// - `page_table`： 页表，用于完成映射操作
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    /// 将整个逻辑段从页表中取消映射，
    /// 具体的实现是遍历逻辑段所包含的所有虚拟页号，
    /// 并调用 `unmap_one` 方法将每个虚拟页号取消映射
    /// 
    /// 参数：
    /// - `page_table`： 页表，用于完成取消映射操作
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// 将数据从传入的数组切片拷贝到当前逻辑段映射到的物理内存中，
    /// 该复制过程确保了数据的正确对齐和分页处理：
    /// 切片 data 中的数据大小不超过当前逻辑段的总大小，
    /// 且切片中的数据会被对齐到逻辑段的开头，
    /// 然后根据当前逻辑段中的虚拟页号范围及其映射关系，使用迭代器的 `step` 方法逐页拷贝到实际的物理页帧。
    /// 
    /// 参数：
    /// - `page_table`： 页表，用于完成映射操作
    /// - `data`： 需要被拷贝的数据数组切片
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let data_len = data.len();

        loop {
            let src = &data[start..data_len.min(start+PAGE_SIZE)];
            let dest = &mut page_table.translate(current_vpn).unwrap().ppn().get_bytes_array()[..src.len()];
            dest.copy_from_slice(src);
            start += PAGE_SIZE;
            if start > data_len {
                break;
            }
            current_vpn.step();
        }
    }
}

/// 内存集，代表一个完整的地址空间区域，
/// 具有两个成员变量：
/// - `page_table`： `PageTable`，表示该内存集所使用的页表，
///   用于管理虚拟地址到物理地址的映射关系
/// - `areas`： `Vec<MapArea>`，表示该内存集中包含的所有逻辑段，
///  每个逻辑段都包含一段连续地址的物理内存
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// 创建一个新的空内存集，
    /// 该内存集包含一个新的页表和一个空的逻辑段列表
    /// 
    /// 返回值：
    /// - `Self`： 返回一个新的 `MemorySet` 实例，表示创建的内存集
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// 向内存集中添加一个新的逻辑段，
    /// 该方法会调用 `MapArea.map` 方法将传入的逻辑段映射到内存集的页表 `page_table` 中，
    /// 调用 `MapArea.copy_data` 方法复制数据（如果有的话）到映射的物理内存中，
    /// 并将其添加到逻辑段列表 `areas` 中
    /// 
    /// 参数：
    /// - `map_area`： 需要添加的逻辑段
    /// - `data`： 逻辑段对应的初始数据
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// 为内存集添加一个新的 `Framed` 类型的逻辑段，
    /// 该方法会调用 `push` 方法将传入的起始和结束虚拟地址、
    /// 映射类型 `Framed` 和权限创建一个新的逻辑段并添加到内存集中
    /// 
    /// 参数：
    /// - `start_va`： 逻辑段的起始虚拟地址
    /// - `end_va`： 逻辑段的结束虚拟地址
    /// - `permission`： 逻辑段的权限，可以是读、写、执行和用户权限的组合
    pub fn insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission){
        self.push(MapArea::new(
            start_va,
            end_va,
            MapType::Framed,
            permission,
        ), None);
    }

    pub fn map_trampoline(&mut self) {
        todo!()
    }

    /// 创建一个新的内核内存集，
    /// 该内存集包含内核代码段、只读数据段、数据段、BSS 段和物理内存映射段，
    /// 分别使用 `MemorySet.push` 方法将这些逻辑段添加到内存集中，映射的类型为 `Identical`，
    /// 权限根据不同的逻辑段进行设置
    /// 
    /// 返回值：
    /// - `Self`： 返回一个新的 `MemorySet` 实例，表示创建的内核内存集
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();
        debug!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        debug!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        debug!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        debug!(".bss [{:#x}, {:#x})", sbss_with_stack as usize, ebss as usize);

        debug!("mapping .text section");
        memory_set.push(MapArea::new(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::X
        ), None);

        debug!("mapping .data section");
        memory_set.push(MapArea::new(
            (sdata as usize).into(),
            (edata as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ), None);

        debug!("mapping .bss section");
        memory_set.push(MapArea::new(
            (sbss_with_stack as usize).into(),
            (ebss as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ), None);

        debug!("mapping physical memory");
        memory_set.push(MapArea::new(
            (ekernel as usize).into(),
            MEMORY_END.into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ), None);

        memory_set
    }

    /// 从 ELF 文件数据创建用户态应用程序的地址空间，
    /// 该方法会解析 ELF 格式的应用程序，并根据其程序头表创建对应的内存映射，
    /// 最终构建出完整的用户态地址空间布局
    ///
    /// ```text
    ///     High Address
    ///   ┌─────────────────┐
    ///   │   Trampoline    │ <- TRAMPOLINE (highest)
    ///   ├─────────────────┤
    ///   │   TrapContext   │ <- TRAP_CONTEXT
    ///   ├─────────────────┤
    ///   │                 │
    ///   │   (unmapped)    │
    ///   │                 │
    ///   ├─────────────────┤
    ///   │  sbrk mapping   │ <- user_stack_top (zero-length, for heap expansion)
    ///   ├─────────────────┤
    ///   │                 │
    ///   │   User Stack    │ <- USER_STACK_SIZE
    ///   │   (grows down)  │    U+R+W, Framed
    ///   │                 │
    ///   ├─────────────────┤ <- user_stack_bottom
    ///   │   guard page    │ <- PAGE_SIZE (unmapped)
    ///   ├─────────────────┤ <- max_end_va
    ///   │      .bss       │    U+R+W
    ///   ├─────────────────┤
    ///   │      .data      │    U+R+W
    ///   ├─────────────────┤
    ///   │    .rodata      │    U+R
    ///   ├─────────────────┤
    ///   │      .text      │    U+R+X
    ///   └─────────────────┘ <- 0x10000
    ///     Low Address
    /// ```
    /// 地址空间布局（从高地址到低地址）：
    /// 1. **应用程序高半部分** (接近 2^64 - 256GiB)：
    ///    - Trampoline：跳板页，用于在用户态和内核态之间切换
    ///    - TrapContext：用于保存 Trap 上下文，权限为 R+W
    ///
    /// 2. **用于 sbrk 的初始映射**：
    ///    零长度映射，标记堆的起始位置，用于后续的堆空间扩展
    ///
    /// 3. **User Stack**（用户栈）：
    ///    大小为 USER_STACK_SIZE，权限为 U+R+W，采用 Framed 映射
    ///
    /// 4. **guard page**（保护页）：
    ///    位于应用段结束后的一个页面，用于分隔应用数据与用户栈
    ///
    /// 5. **应用程序低半部分** (0x10000 开始)：
    ///    - .bss 段：未初始化数据段，权限为 U+R+W
    ///    - .data 段：可读写数据段，权限为 U+R+W
    ///    - .rodata 段：只读数据段，权限为 U+R
    ///    - .text 段：代码段，权限为 U+R+X
    ///
    /// 参数：
    /// - `elf_data`： ELF 文件的原始字节数据
    ///
    /// 返回值：
    /// - `(Self, usize, usize)`：
    ///   - 第一个值：构建好的用户态内存集
    ///   - 第二个值：用户栈顶地址
    ///   - 第三个值：应用程序入口点地址
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        // 创建一个新的空内存集
        let mut memory_set = Self::new_bare();
        // 映射跳板页，用于在用户态和内核态之间切换
        memory_set.map_trampoline();

        // 解析 ELF 文件
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        // 检查 ELF 魔数是否正确
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

        // 获取程序头表的数量
        let ph_count = elf_header.pt2.ph_count();
        // 记录所有程序段的最大结束虚拟页号，用于后续确定用户栈的位置
        let mut max_end_vpn = VirtPageNum(0);

        // 遍历所有程序头，映射可加载段（.text, .rodata, .data, .bss）
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            // 只处理类型为 LOAD 的段
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                // 获取段的起始和结束虚拟地址
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                // 根据段的标志位设置映射权限，所有用户段都需要 U 权限
                let mut map_permission = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_permission |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_permission |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_permission |= MapPermission::X;
                }

                // 创建逻辑段，使用 Framed 映射类型
                let map_area = MapArea::new(
                    start_va,
                    end_va,
                    MapType::Framed,
                    map_permission
                );
                // 更新最大结束虚拟页号
                max_end_vpn = map_area.vpn_range.get_end();
                // 将段添加到内存集中，并复制 ELF 文件中的数据
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset()+ph.file_size()) as usize])
                );
            }
        }

        // 计算用户栈的位置：在应用段结束后留出一个保护页（guard page），然后放置用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // 跳过 guard page（保护页）
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;

        // 映射用户栈，权限为 U+R+W
        memory_set.push(MapArea::new(
            user_stack_bottom.into(),
            user_stack_top.into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W | MapPermission::U
        ), None);

        // 在用户栈顶创建一个零长度的映射，用于 sbrk 系统调用的堆空间管理
        // 这个映射标记了堆的起始位置，后续可以通过 sbrk 扩展堆空间
        memory_set.push(MapArea::new(
            user_stack_top.into(),
            user_stack_top.into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W | MapPermission::U,
        ), None);

        // 映射 TrapContext，用于在 Trap 发生时保存用户态的上下文信息
        memory_set.push(MapArea::new(
            TRAP_CONTEXT.into(),
            TRAMPOLINE.into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W
        ), None);

        // 返回内存集、用户栈顶地址和应用程序入口点地址
        (memory_set, user_stack_top, elf.header.pt2.entry_point() as usize)
    }
}