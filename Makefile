
ARCH ?= x86_64

CARGO ?= cargo
PYTHON ?= python3
QEMU ?= qemu-system-x86_64

BOOT := boot

BOOT_PROJECT := chos-$(BOOT)

FLAGS := -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem

include boot/arch/$(ARCH).mk

.PHONY: all build expand run test

all: build

build:
	$(CARGO) build -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

expand:
	$(CARGO) expand -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

run:
	$(CARGO) run -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

test:
	$(CARGO) test -p $(BOOT_PROJECT) $(FLAGS) $(BOOT_FLAGS)

clean:
	$(CARGO) clean