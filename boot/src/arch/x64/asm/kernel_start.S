/*
    %rdi = kernel info struct (see /docs/boot.txt, or KernelBootInfo in /lib/chos-boot-defs/src/lib.rs)
    %rsi = apic id
*/
.global kernel_start
.type kernel_start, @function
kernel_start:
    jmp *(%rdi)
    cli
0:  hlt
    jmp 0b
    ud2
