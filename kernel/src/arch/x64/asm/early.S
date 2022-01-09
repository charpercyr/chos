
/*
    rdi = APIC ID (pass through)
    rsi = Kernel Args (pass through)
    rdx = stack
*/

.global enter_kernel_main
.type enter_kernel_main, @function
enter_kernel_main:
    mov %rdx, %rsp
    push $0
    mov $0, %rbp
    call kernel_main
    ud2
.size enter_kernel_main, . - enter_kernel_main
