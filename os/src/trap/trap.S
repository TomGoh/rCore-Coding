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

    .section .text # 指定代码段
    .globl __alltraps # 定义全局符号 __alltraps
    .globl __restore # 定义全局符号 __restore
    .align 2 # 对齐到 4 字节边界

/*
 * 保存现场到内核栈，并调用 Rust 函数处理
 * 1. 交换 sp 和 sscratch
 * 2. 在内核栈上分配 TrapContext 空间
 * 3. 保存通用寄存器
 * 4. 保存 sstatus、sepc 和用户栈指针
 * 5. 调用 Rust 函数处理
 */
__alltraps:
    # 保存用户态 sp 到 sscratch，切换到内核栈
    # 具体是通过将 sp 和 sscratch 交换实现
    # before: sp->user stack, sscratch->kernel stack
    # after: sp->kernel stack, sscratch->user stack
    csrrw sp, sscratch, sp
    # 现在 sp->内核栈, sscratch->用户栈
    # 在内核栈上分配 TrapContext 空间
    # TrapContext 结构体的大小为：
    # 通用寄存器数组的 32 个 usize + Sstatus + Sepc
    # 一共 34 个 usize
    addi sp, sp, -34*8
    # 保存通用寄存器
    sd x1, 1*8(sp)
    # 跳过 sp(x2)，稍后保存
    sd x3, 3*8(sp)
    # 跳过 tp(x4)，应用程序不使用它
    # 保存 x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 可以自由使用 t0/t1/t2，因为它们已保存在内核栈上
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # 从 sscratch 读取用户栈指针并保存到内核栈
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # 设置 trap_handler(cx: &mut TrapContext) 的输入参数
    mv a0, sp
    call trap_handler

/*
 * 恢复现场并返回用户态，在 trap_handler 返回后执行
 * 6. 恢复 sstatus、sepc 和用户栈指针
 * 7. 恢复通用寄存器
 * 8. 释放内核栈上的 TrapContext 空间
 * 9. 交换 sp 和 sscratch，返回用户态
 */
__restore:
    # __restore 有两种情况会被调用：
    # 情况1: 通过 __restore 开始运行应用程序
    # 情况2: 处理陷入后返回用户态
    # 两种情况都需要恢复现场
    # 通过 a0 寄存器传入的参数是 TrapContext 的地址
    mv sp, a0
    # 现在 sp->内核栈(已分配), sscratch->用户栈
    # 恢复 sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # 恢复通用寄存器（除了 sp/tp）
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 释放内核栈上的 TrapContext 空间
    addi sp, sp, 34*8
    # 现在 sp->内核栈, sscratch->用户栈
    csrrw sp, sscratch, sp
    sret
