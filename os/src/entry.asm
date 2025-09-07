    .section .text.entry # 将之后的代码都放置在名为 .text.entry 的段落中，作为入口点
    .global _start # 声明一个全局符号_start，可以被其他目标文件使用 
_start:
    la sp, boot_stack_top # 初始化栈指针寄存器 sp，指向 boot_stack_top 
    call rust_main    # 调用 rust_main 函数，开始执行 Rust 代码

    .section .bss.stack # 将之后的未初始化数据放置在名为 .bss.stack 的段落中 
    .global boot_stack_lower_bound # 声明一个全局符号 boot_stack_lower_bound 
boot_stack_lower_bound:
    .space 4096 * 16 # 分配 4096 * 16 字节的空间，用于栈 
    .global boot_stack_top # 声明一个全局符号 boot_stack_top 
boot_stack_top: