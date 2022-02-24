
ENTRY(entry);

SECTIONS {
    .rodata : {
        *(.rodata*)

        __chos_static_mod_start = .;
        *(.chos.module*)
        __chos_static_mod_end = .;
    }

    . = ALIGN(4k);

    .text : {
        *(.text*)
        *(.plt*)
    }

    . = ALIGN(4k);

    .data : {
        *(.data*)
        *(.got.plt*)
    }

    .tdata : {
        *(.tdata*)
    }

    . = ALIGN(4k);

    .bss : {
        *(.bss*)
        *(COMMON)
    }

    .tbss : {
        *(.tbss*)
    }
}
