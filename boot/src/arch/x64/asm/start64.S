
.section .inittext, "ax"
.code64

.global _start64
.type _start64, @function
_start64:
    mov $stack64_begin, %rsp
    push $0

    mov $0, %rbp

    mov $0x0, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    mov %rbx, %rdi
    call boot_main
0:
    hlt
    jmp 0b
.size _start64, . - _start64

.section .bss
.align 16
stack64_end:
.skip 4096 * 8 // 8 pages
stack64_begin: