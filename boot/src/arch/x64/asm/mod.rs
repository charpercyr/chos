use chos_lib::include_asm;

include_asm!(
    "kernel_start.S",
    "./mpstart.S",
    "./multiboot.S",
    "./start32.S",
    "./start64.S",
    "./vga.S",
);
