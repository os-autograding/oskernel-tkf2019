# 代码空间
    .section .text.entry
    .globl _start
# 操作系统入口函数
_start:
    # 堆栈初始化
    la sp, boot_stack_top
    # 进入rust主函数
    call rust_main

    # 回忆：bss 段是 ELF 文件中只记录长度，而全部初始化为 0 的一段内存空间
    # 这里声明字段 .bss.stack 作为操作系统启动时的栈
    .section .bss.stack
    .global boot_stack
boot_stack:
    # 16K 启动栈大小
    .space 4096 * 16
    .global boot_stack_top
boot_stack_top:
    # 栈结尾