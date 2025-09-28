.altmacro
# 定义宏，负责保存和恢复 s0~s11 寄存器
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
# 定义宏，负责保存和恢复 s0~s11 寄存器
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm
    .section .text
    .globl __switch
__switch:
    # 调用方式：
    # __switch(
    #     current_task_cx_ptr: *mut TaskContext,
    #     next_task_cx_ptr: *const TaskContext
    # )
    # 其中，current_task_cx_ptr 和 next_task_cx_ptr 分别指向当前任务和下一个任务的上下文结构体
    # 寄存器与参数的对应关系：
    # a0 -> current_task_cx_ptr
    # a1 -> next_task_cx_ptr
    #
    # 1. 保存当前任务的上下文到 current_task_cx_ptr 指向的结构体
    # 首先，保存 sp 到结构体 current_task_cx_ptr
    sd sp, 8(a0)
    # 保存 ra 到结构体 current_task_cx_ptr
    sd ra, 0(a0)
    # 保存 s0~s11 到结构体 current_task_cx_ptr
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr
    # 2. 恢复下一个任务的上下文，从 next_task_cx_ptr 指向的结构体加载
    # 首先，从 next_task_cx_ptr 恢复 ra
    ld ra, 0(a1)
    # 从 next_task_cx_ptr 恢复 s0~s11
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # 最后，从 next_task_cx_ptr 恢复 sp
    ld sp, 8(a1)
    ret

