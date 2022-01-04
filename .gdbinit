set confirm off
set pagination off
file target/x86_64-chos/debug/chos-boot.elf
add-symbol-file target/x86_64-chos/debug/chos.elf -o 0xffff808000000000
target remote tcp::1234
b boot_main
b chos_boot::arch::x64::panic::panic
b chos::panic::panic
set confirm on
c