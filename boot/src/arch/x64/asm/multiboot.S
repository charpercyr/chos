.set MAGIC, 0xe85250d6
.set ARCH, 0
.set LEN, multiboot_end - multiboot_start
.set CHECKSUM, 0x100000000 - (MAGIC + ARCH + LEN)

.section .multiboot

.set multiboot_start, .
.global multiboot_header
multiboot_header:
// HEADER
.long MAGIC
.long ARCH
.long LEN
.long CHECKSUM

// END TAG
.long 0
.long 8

.set multiboot_end, .
