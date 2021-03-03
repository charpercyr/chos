

.set PRESENT, (1 << 0)
.set WRITEABLE, (1 << 1)
.set NOCACHE, (1 << 4) | (1 << 5)
.set HUGEPAGE, (1 << 7)

.section .inittext, "ax"
.code32

.global _start32
.type _start32, @function
_start32:
    mov $stack32_begin, %esp

    push %ebx

    call asm_vga_clear

    mov %eax, %edi
    call check_multiboot
    call check_cpuid
    call check_64

    call enable_paging
    call load_gdt

    pop %ebx

    ljmpl $0x8, $_start64
0:
    hlt
    jmp 0b
.size _start32, . - _start32

.type load_gdt, @function
load_gdt:
    mov $gdt_ptr, %eax
    movw $(.gdt_end - gdt - 1), 0(%eax)
    movl $gdt, 2(%eax)
    lgdt (%eax)
.size load_gdt, . - load_gdt

.type enable_paging, @function
enable_paging:
    // Map first 4GB
    xor %eax, %eax
    or $(PRESENT | WRITEABLE | HUGEPAGE | NOCACHE | (0 << 30)), %eax
    mov %eax, (p3_table + 0x0)

    xor %eax, %eax
    or $(PRESENT | WRITEABLE | HUGEPAGE | NOCACHE | (1 << 30)), %eax
    mov %eax, (p3_table + 0x8)

    xor %eax, %eax
    or $(PRESENT | WRITEABLE | HUGEPAGE | NOCACHE | (2 << 30)), %eax
    mov %eax, (p3_table + 0x10)

    xor %eax, %eax
    or $(PRESENT | WRITEABLE | HUGEPAGE | NOCACHE | (3 << 30)), %eax
    mov %eax, (p3_table + 0x18)

    mov $p3_table, %eax
    or $(PRESENT | WRITEABLE), %eax
    mov %eax, p4_table

    mov $(p4_table ), %eax
    mov %eax, %cr3

    mov %cr4, %eax
    or $(1 << 5), %eax
    mov %eax, %cr4

    mov $0xC0000080, %ecx
    rdmsr
    or $(1 << 8), %eax
    wrmsr

    mov %cr0, %eax
    or $(1 << 31), %eax
    mov %eax, %cr0

    ret
.size enable_paging, . - enable_paging

.type check_multiboot, @function
check_multiboot:
    cmp $0x36d76289, %edi
    mov $msg_no_multiboot, %edi
    jne err_loop
    ret
.size check_multiboot, . - check_multiboot

.type check_cpuid, @function
check_cpuid:
    pushfd
    pop %eax

    mov %eax, %ecx

    xor $(1 << 21), %eax

    push %eax
    popfd

    pushfd
    pop %eax

    push %ecx
    popfd

    mov $msg_no_cpuid, %edi
    cmp %ecx, %eax
    je err_loop
    
    ret
    
.size check_cpuid, . - check_cpuid

.type check_64, @function
check_64:
    mov $msg_no_64, %edi
    mov $0x80000000, %eax
    cpuid
    cmp $0x80000001, %eax
    jb err_loop
    mov $0x80000001, %eax
    cpuid
    test $(1 << 29), %edx
    jz err_loop
    ret
.size check_64, . - check_64

// %edi = null terminated string
err_loop:
    mov $0x4, %si
    call asm_vga_set_str
0:
    hlt
    jmp 0b

.section .rodata
msg_no_multiboot: .string "No Multiboot"; .size msg_no_multiboot, . - msg_no_multiboot
msg_no_cpuid: .string "No CPUID"; .size msg_no_cpuid, . - msg_no_cpuid
msg_no_64: .string "64 bit not supported"; .size msg_no_64, . - msg_no_64

gdt:
.quad 0
.quad (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)
.size gdt, . - gdt
.gdt_end:

.section .bss
.align 4096
p4_table:
.skip 4096
p3_table:
.skip 4096
stack32_end:
.skip 64
stack32_begin:
gdt_ptr:
.skip 24