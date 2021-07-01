.section .rodata

.align 16

.set START, .
.set BASE, 0x8000

.global MPSTART_16
.type MPSTART_16, @object
MPSTART_16:
    .code16
    cli

    ljmp $0, $(0f - START + BASE)
0:
    // Clear segment registers
    xorw %ax, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs

    // Load 32-bit GDT
    movl $(MPSTART_GDT32_PTR - START + BASE), %eax
    lgdt (%eax)

    // Enable protected mode
    mov %cr0, %eax
    or $1, %al
    mov %eax, %cr0

    // Jmp to protected mode, 2nd entry (code segment)
    ljmpl $0x8, $(.Lmpstart32 - START + BASE)
0:
    hlt
    jmp 0b
.Lmpstart32:
    .code32

    // Set segment registers to 3rd entry (data segment)
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %ss

    // Load 64-bit GDT
    movl $(MPSTART_GDT64_PTR - START + BASE), %eax
    lgdt (%eax)

    // Set page table
    movl $MPSTART_PDT4, %eax
    movl (%eax), %eax
    movl %eax, %cr3

    // Enable long mode
    movl %cr4, %eax
    orl $(1 << 5), %eax
    movl %eax, %cr4

    movl $0xC0000080, %ecx
    rdmsr
    orl $((1 << 8) | (1 << 11)), %eax
    wrmsr

    movl %cr0, %eax
    orl $(1 << 31), %eax
    movl %eax, %cr0

    // Jmp to long mode
    ljmpl $0x8, $(.Lmpstart64 - START + BASE)
0:
    hlt
    jmp 0b
.Lmpstart64:
    .code64
    // Clear segment registers
    movw $0x0, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs
    movw %ax, %ss


    // Setup stack for CPU
    // ebx = APIC ID + 1
    // eax = Stack base
    // ecx = stack stride
    // rsp = eax + ecx * ebx
    mov $0x1, %rax
    cpuid
    and $0xff000000, %ebx
    shr $24, %ebx
    inc %ebx

    mov $MPSTART_STACK_BASE, %eax
    mov (%rax), %eax
    mov $MPSTART_STACK_STRIDE, %ecx
    mov (%rcx), %ecx

    imul %ebx, %ecx
    lea (%rax, %rcx), %rsp
    push $0

    mov $0, %rbp

    movl $secondary_main, %eax
    callq *%rax
0:
    hlt
    jmp 0b
.size MPSTART_16, . - MPSTART_16

.align 16
.type MPSTART_GDT32, @object
MPSTART_GDT32:
.quad 0x0 // NULL ENTRY
.quad 0x00cf9a000000ffff // flat code
.quad 0x008f92000000ffff // flat data
.size MPSTART_GDT32, . - MPSTART_GDT32

.type MPSTART_GDT32_PTR, @object
MPSTART_GDT32_PTR:
.word . - MPSTART_GDT32 - 1
.long MPSTART_GDT32 - START + BASE
.size MPSTART_GDT32_PTR, . - MPSTART_GDT32_PTR

.align 16
.type MPSTART_GDT64, @object
MPSTART_GDT64:
.quad 0
.quad (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)
.size MPSTART_GDT64, . - MPSTART_GDT64

.type MPSTART_GDT64_PTR, @object
MPSTART_GDT64_PTR:
.word . - MPSTART_GDT64 - 1
.quad MPSTART_GDT64 - START + BASE
.size MPSTART_GDT64_PTR, . - MPSTART_GDT64_PTR

.global MPSTART_16_LEN
.type MPSTART_16_LEN, @object
MPSTART_16_LEN: .quad . - MPSTART_16
.size MPSTART_16_LEN, . - MPSTART_16_LEN