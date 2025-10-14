/* 指定二进制文件的目标架构 */
OUTPUT_ARCH(riscv)

/* 设置程序入口点 - 程序启动时执行的第一条指令 */
ENTRY(_start)

/* 定义内核在物理内存中加载的基地址 */
/* 这是 RISC-V 系统的标准内核加载地址 */
BASE_ADDRESS = 0x80200000;

/* 定义内存布局段 */
SECTIONS
{
    /* 将位置计数器设置为基地址 */
    . = BASE_ADDRESS;
    
    /* 标记整个内核映像开始位置的符号 */
    skernel = .;

    /* 代码段 - 包含可执行代码 */
    stext = .;  /* 标记代码段开始位置的符号 */
    .text : {
        /* 入口点代码放在最前面 - 包含 _start 符号 */
        *(.text.entry)
        /* 对齐到 4KB 页边界以便 trampoline 代码隔离 */
        . = ALIGN(4K);
        /* Trampoline 代码 - 用于用户态和内核态切换的跳板 */
        strampoline = .;  /* 标记 trampoline 代码开始位置的符号 */
        *(.text.trampoline);
        /* 对齐到 4KB 页边界 */
        . = ALIGN(4K);
        /* 所有其他可执行代码段 */
        *(.text .text.*)
    }

    /* 对齐到 4KB 页边界以便内存管理 */
    . = ALIGN(4K);
    etext = .;    /* 标记代码段结束位置的符号 */
    
    /* 只读数据段 - 包含常量和字符串字面量 */
    srodata = .;  /* 标记只读数据段开始位置的符号 */
    .rodata : {
        /* 标准只读数据段 */
        *(.rodata .rodata.*)
        /* 小型只读数据段（RISC-V 特有） */
        *(.srodata .srodata.*)
    }

    /* 对齐到 4KB 页边界 */
    . = ALIGN(4K);
    erodata = .;  /* 标记只读数据段结束位置的符号 */
    
    /* 数据段 - 包含已初始化的全局变量和静态变量 */
    sdata = .;    /* 标记数据段开始位置的符号 */
    .data : {
        /* 标准已初始化数据段 */
        *(.data .data.*)
        /* 小型已初始化数据段（RISC-V 特有） */
        *(.sdata .sdata.*)
    }

    /* 对齐到 4KB 页边界 */
    . = ALIGN(4K);
    edata = .;    /* 标记数据段结束位置的符号 */
    
    /* BSS 段 - 包含未初始化的全局变量和静态变量 */
    /* 这些变量将在运行时被清零 */
    sbss_with_stack = .;  /* 标记包含栈的 BSS 段开始位置的符号 */
    .bss : {
        /* 内核栈空间 - 放在 BSS 段开头 */
        *(.bss.stack)
        sbss = .;     /* 标记一般 BSS 段开始位置的符号 */
        /* 标准未初始化数据段 */
        *(.bss .bss.*)
        /* 小型未初始化数据段（RISC-V 特有） */
        *(.sbss .sbss.*)
    }

    /* 对齐到 4KB 页边界 */
    . = ALIGN(4K);
    ebss = .;     /* 标记 BSS 段结束位置的符号 */
    ekernel = .;  /* 标记整个内核映像结束位置的符号 */

    /* 丢弃段 - 从最终二进制文件中移除不需要的段 */
    /DISCARD/ : {
        /* 异常处理帧信息 - 内核不需要 */
        *(.eh_frame)
    }
}