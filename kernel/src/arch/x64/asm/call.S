
/*
    Args:
    rdi = function pointer
    rsi = stack
    rdx = arg0
    rcx = arg1
    r8 = arg2
    r9 = arg3
*/

.global call_with_stack
.type call_with_stack, @function
call_with_stack:
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
