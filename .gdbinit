tui enable
focus cmd
set confirm off
set pagination off
set print pretty on
set input-radix 16
add-symbol-file target/x86_64-chos/debug/chos.elf -o 0xffff808000000000
target remote tcp::1234
b chos::panic::panic
set confirm on
c