#!/bin/zsh

rust-gdb \
	-ex "set confirm off" \
	-ex "set pagination off" \
	-ex "file target/x86_64-chos/debug/chos-boot.elf" \
	-ex "add-symbol-file target/x86_64-chos/debug/chos.elf -o 0xffff818000000000" \
	-ex "target remote tcp::1234" \
	-ex "b boot_main" \
	-ex "b chos_boot::arch::x64::panic::panic" \
	-ex "b chos::panic::panic" \
	-ex "set confirm on" \
	-ex "c"
