.altmacro
# 定义宏，负责保存和恢复通用寄存器
.macro SAVE_GP n
    # 接受参数 n，保存通用寄存器 x\n 到栈上
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    # 接受参数 n，从栈上恢复通用寄存器 x\n
    ld x\n, \n*8(sp)
.endm

    .section .text.trampoline # 指定代码段为跳板段 .text.trampoline
    .globl __alltraps # 定义全局符号 __alltraps
    .globl __restore # 定义全局符号 __restore
    .align 2 # 对齐到 4 字节边界

/*
 * 保存用户态上下文并跳转到内核的 trap 处理函数
 * 当 trap 发生时，硬件会跳转到此处（通过 stvec 寄存器设置）
 *
 * 主要步骤：
 * 1. 交换 sp 和 sscratch，使 sp 指向用户空间的 TrapContext
 * 2. 保存所有通用寄存器到 TrapContext
 * 3. 保存 sstatus、sepc 和用户栈指针
 * 4. 从 TrapContext 加载内核页表、内核栈、trap_handler 地址
 * 5. 切换到内核页表和内核栈
 * 6. 跳转到 trap_handler 处理 trap
 */
__alltraps:
    # 交换 sp 和 sscratch
    # 执行前：sp -> 用户栈，sscratch -> TrapContext（在用户空间中）
    # 执行后：sp -> TrapContext，sscratch -> 用户栈
    csrrw sp, sscratch, sp

    # 现在 sp 指向用户空间的 TrapContext，sscratch 保存用户栈指针
    # TrapContext 在用户地址空间中已经预先分配好
    # 开始保存用户态的寄存器到 TrapContext

    # 保存 x1 (ra: 返回地址寄存器)
    sd x1, 1*8(sp)
    # 跳过 x2 (sp)，稍后会保存 sscratch 中的用户栈指针
    # 保存 x3 (gp: 全局指针寄存器)
    sd x3, 3*8(sp)
    # 跳过 x4 (tp)，应用程序不使用线程指针
    # 保存 x5~x31（使用宏批量保存）
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr

    # 现在可以自由使用 t0/t1/t2，因为它们已经保存在 TrapContext 中

    # 保存 sstatus 寄存器（用户态状态）
    csrr t0, sstatus
    # 保存 sepc 寄存器（用户态程序计数器，trap 发生时的 PC）
    csrr t1, sepc
    # 将 sstatus 保存到 TrapContext 的偏移 32*8
    sd t0, 32*8(sp)
    # 将 sepc 保存到 TrapContext 的偏移 33*8
    sd t1, 33*8(sp)

    # 保存用户栈指针
    # 从 sscratch 读取用户栈指针（之前通过 csrrw 交换过来的）
    csrr t2, sscratch
    # 将用户栈指针保存到 TrapContext 的偏移 2*8（对应 x2/sp）
    sd t2, 2*8(sp)

    # 从 TrapContext 中加载内核态所需的信息
    # 这些字段在创建任务时由内核预先填充到 TrapContext 中

    # 加载内核页表令牌（satp 寄存器的值）
    ld t0, 34*8(sp)
    # 加载 trap_handler 函数的地址
    ld t1, 36*8(sp)
    # 加载内核栈指针
    ld sp, 35*8(sp)

    # 切换到内核地址空间
    # 将内核token，也就是内核页表起始地址，写入 satp 寄存器
    csrw satp, t0
    # 刷新 TLB，确保使用新的页表进行地址转换
    sfence.vma

    # 跳转到 trap_handler 处理函数（间接跳转）
    # 注意：此时 sp 已经指向内核栈，可以安全地调用 Rust 函数
    jr t1

/*
 * 恢复用户态上下文并返回用户态
 * 该函数有两种调用场景：
 * 1. 首次启动应用程序时，由内核主动调用
 * 2. 处理完 trap 后，从 __alltraps 跳转到此处继续执行
 *
 * 主要步骤：
 * 1. 切换到用户地址空间（切换页表）
 * 2. 设置 sscratch 寄存器指向用户空间中的 TrapContext
 * 3. 从 TrapContext 恢复 sstatus 和 sepc
 * 4. 恢复通用寄存器
 * 5. 恢复用户栈指针
 * 6. 通过 sret 返回用户态
 */
__restore:
    # 函数参数：
    # a0: 指向用户空间中 TrapContext 的指针（常量地址，如 TRAP_CONTEXT）
    # a1: 用户地址空间的页表令牌（satp 寄存器的值）

    # 切换到用户地址空间
    # 将用户页表token，也就是用户页表起始地址，写入 satp 寄存器，完成页表切换
    csrw satp, a1
    # 刷新 TLB，确保地址转换使用新的页表
    sfence.vma

    # 设置 sscratch 寄存器
    # sscratch 将保存 TrapContext 的地址，当发生 trap 时，
    # __alltraps 会通过 csrrw 指令交换 sp 和 sscratch，
    # 从而让 sp 指向 TrapContext，实现上下文保存
    csrw sscratch, a0

    # 将 sp 临时设置为 TrapContext 的地址
    # 接下来将基于 sp 从 TrapContext 中恢复寄存器
    mv sp, a0

    # 现在 sp 指向用户空间中的 TrapContext，开始恢复上下文

    # 从 TrapContext 恢复 sstatus 和 sepc
    # sstatus: 保存的用户态状态寄存器
    ld t0, 32*8(sp)
    # sepc: 保存的用户态程序计数器（返回地址）
    ld t1, 33*8(sp)
    # 写入 sstatus，恢复用户态的中断使能等状态
    csrw sstatus, t0
    # 写入 sepc，设置 sret 指令的返回地址
    csrw sepc, t1

    # 恢复通用寄存器（除了 x0/sp/tp）
    # x0: 硬件固定为 0，无需恢复
    # sp: 稍后单独恢复
    # tp: 应用程序不使用

    # 恢复 x1 (ra: 返回地址寄存器)
    ld x1, 1*8(sp)
    # 恢复 x3 (gp: 全局指针寄存器)
    ld x3, 3*8(sp)
    # 恢复 x5~x31（使用宏批量恢复）
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr

    # 恢复用户栈指针
    # 从 TrapContext 的偏移 2*8 处加载用户栈指针
    # 注意：这条指令执行后，sp 将指向用户栈，不再指向 TrapContext
    ld sp, 2*8(sp)

    # 执行 sret 指令返回用户态
    # sret 会做以下事情：
    # 1. 将特权级从 S 模式切换到 U 模式
    # 2. 将 pc 设置为 sepc 的值（跳转到用户程序）
    # 3. 将 sstatus.SPP 位清零（标记当前为用户态）
    sret