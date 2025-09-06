    .section .text.entry /* 将之后的代码都放置在名为 .text.entry 的段落中，作为入口点 */
    .global _start /* 声明一个全局符号_start，可以被其他目标文件使用 */
_start:
    li x1, 100