
ENTRY(_start32)

SECTIONS {
    . = 1M;

    .text : ALIGN(4K) {
        *(.multiboot*)
        *(.inittext*)
        *(.text*)
    }

    .data : ALIGN(4K) {
        *(.data*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata*)
    }

    .bss : ALIGN(4K) {
        *(COMMON)
        *(.bss*)
    }
}
