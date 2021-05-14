
/*
    %rdi = kernel info struct (see /docs/boot.txt)
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
