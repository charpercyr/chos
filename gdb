#!/bin/zsh

rust-gdb \
		-ex "set pagination off" \
		-ex "file target/x86_64-chos/debug/chos-boot.elf" \
		-ex "target remote tcp::1234" \
		-ex "b boot_main" \
		-ex "c"