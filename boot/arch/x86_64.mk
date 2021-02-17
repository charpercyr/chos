
BOOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

BOOT_FLAGS := --target $(BOOT_DIR)$(ARCH)-chos-boot.json
BOOT_RUSTC_FLAGS :=