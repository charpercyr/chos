use chos_lib::include_asm;

include_asm!("./multiboot.S", "./start32.S", "./start64.S", "./vga.S",);
