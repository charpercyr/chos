
KERNEL_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

KERNEL_FLAGS := --target $(KERNEL_DIR)$(ARCH)-chos-kernel.json
KERNEL_RUSTC_FLAGS :=