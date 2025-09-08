OUTPUT_ARCH(riscv)
ENTRY(_start) # 将 lib.rs 中定义的用户空间起始函数挂载

BASE_ADDRESS = 0x80400000; # 与 os 的起始物理地址不同，用户的应用程序会被加载到这个地址上

/* 数据段和代码段的定义与系统空间中的一致 */
SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        start_bss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        end_bss = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}