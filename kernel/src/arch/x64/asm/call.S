
/*
    Args:
    rdi = function pointer
    rsi = stack
    rdx = arg0
    rcx = arg1
    r8 = arg2
    r9 = arg3
*/

.global __call_with_stack
.type __call_with_stack, @function
__call_with_stack:
    mov %rsi, %rsp
    push $0
    mov $0, %rbp
    mov %rdi, %rax
    mov %rdx, %rdi
    mov %rcx, %rsi
    mov %r8, %rdx
    mov %r9, %rcx
    call *%rax
    ud2
.size __call_with_stack, . - __call_with_stack