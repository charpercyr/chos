
ENTRY(entry);

SECTIONS {
    .text : ALIGN(4k) {
        *(.text*)
    }

    .data : ALIGN(4K) {
        *(.data*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata*)
    }

    .bss : ALIGN(4K) {
        *(.bss*)
    }

    . = ALIGN(4K);

    .got : { *(.got) }
    .dynamic : { *(.dynamic) }

    . = ALIGN(4K);

    .rela.dyn : { *(.rela.dyn) }
    .hash : { *(.hash) }
    .gnu.hash : { *(.gnu.hash) }
}