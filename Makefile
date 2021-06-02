
ARCH ?= x86_64

CARGO ?= cargo
PYTHON ?= python3
QEMU ?= qemu-system-x86_64

BOOT := boot
KERNEL := kernel

BOOT_PROJECT := chos-$(BOOT)
KERNEL_PROJECT := chos

FLAGS := $(FLAGS) -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem

include $(BOOT)/arch/$(ARCH).mk
include $(KERNEL)/arch/$(ARCH).mk

.PHONY: all build build-kernel run test gdb

all: build

build-kernel:
	$(CARGO) build -p $(KERNEL_PROJECT) $(FLAGS) $(KERNEL_FLAGS)

build: build-kernel
	$(CARGO) build -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

run: build-kernel
	$(CARGO) run -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

test:
	$(CARGO) test -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

gdb:
	rust-gdb \
		-tui \
		-ex "set pagination off" \
		-ex "file target/x86_64-chos-boot/debug/chos-boot.elf" \
		-ex "target remote tcp::1234" \
		-ex "b boot_main" \
		-ex "c"

clean:
	$(CARGO) clean
