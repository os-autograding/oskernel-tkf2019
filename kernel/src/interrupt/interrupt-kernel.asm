.altmacro
# 寄存器宽度对应的字节数
.set    REG_SIZE, 8
# Context 的大小
.set    CONTEXT_SIZE, 34

# 宏：将寄存器存到栈上
.macro SAVE_K reg, offset
    sd  \reg, \offset*8(sp)
.endm

.macro SAVE_K_N n
    SAVE_K  x\n, \n
.endm

# 宏：将寄存器从栈中取出
.macro LOAD_K reg, offset
    ld  \reg, \offset*8(sp)
.endm

.macro LOAD_K_N n
    LOAD_K  x\n, \n
.endm

    .section .text
    .global kernel_callback_entry
# 内核中断调用入口
kernel_callback_entry:
    addi    sp, sp, CONTEXT_SIZE*-8

    # 保存通用寄存器，除了 x0（固定为 0）
    SAVE_K    x1, 1
    # 将原来的 sp（sp 又名 x2）写入 2 位置
    addi    x1, sp, 34*8
    SAVE_K    x1, 2
     # 保存 x3 至 x31
    .set    n, 3
    .rept   29
        SAVE_K_N  %n
        .set    n, n + 1
    .endr
    # 取出 CSR 并保存
    csrr    t0, sstatus
    csrr    t1, sepc
    SAVE_K    t0, 32
    SAVE_K    t1, 33

    # 将第一个参数设置为栈顶 便于Context引用访问
    add a0, x0, sp
    # 第二个参数设置为scause
    csrr a1, scause
    # 第三个参数设置为stval
    csrr a2, stval

    # 调用中断回调函数
    call kernel_callback

    # 恢复 CSR
    LOAD_K    s1, 32
    LOAD_K    s2, 33
    csrw    sstatus, s1
    csrw    sepc, s2

    # 恢复通用寄存器
    LOAD_K    x1, 1

    # 恢复 x3 至 x31
    .set    n, 3
    .rept   29
        LOAD_K_N  %n
        .set    n, n + 1
    .endr

    # 恢复 sp（又名 x2）这里最后恢复是为了上面可以正常使用 LOAD 宏
    LOAD_K    x2, 2
    sret