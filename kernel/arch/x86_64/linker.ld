
ENTRY(entry);

SECTIONS {
    . = 0;

    .text : {
        *(.text*)
    }

    .plt : {
        *(.plt)
    }

    . = ALIGN(4K);

    .data : {
        *(.data*)
    }

    .got.plt : {
        *(.got.plt*)
    }

    .bss : {
        *(.bss*)
    }

    . = ALIGN(4K);

    .rodata :  {
        *(.rodata*)
    }

    . = ALIGN(4K);

    .tdata : {
        *(.tdata)
    }

    . = ALIGN(4K);

    .got : { *(.got) }
    .dynamic : { *(.dynamic) }

    . = ALIGN(4K);

    .rela.dyn : { *(.rela.dyn) }
    .hash : { *(.hash) }
    .gnu.hash : { *(.gnu.hash) }
}