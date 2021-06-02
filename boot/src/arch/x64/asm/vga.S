.section .text
.code64

.set VGA_BASE, 0xb8000
.set VGA_LEN, 25 * 80

.global asm_vga_clear
.type asm_vga_clear, @function
asm_vga_clear:
    xor %ecx, %ecx
0:
    movw $0, VGA_BASE(, %rcx, 2)
    inc %ecx
    cmp $VGA_LEN, %ecx
    jne 0b
    ret
.size asm_vga_clear, . - asm_vga_clear


// %edi/%rdi : Null-terminated string
// %si: Color
.global asm_vga_set_str
.type asm_vga_set_str, @function
asm_vga_set_str:
    xor %ecx, %ecx
    shl $8, %si
0:
    xor %ax, %ax
    mov (%rdi, %rcx, 1), %al
    test %al, %al
    jz 1f
    or %si, %ax
    mov %ax, VGA_BASE(, %rcx, 2)
    inc %ecx
    jmp 0b
1:
    ret
.size asm_vga_set_str, . - asm_vga_set_str
