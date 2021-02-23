
use core::mem::MaybeUninit;

use multiboot2 as mb;

use chos_elf::{StringTable, SymbolTable, SymbolType};

static mut ELF_INITIALIZED: bool = false;
static mut SYMBOL_TABLE: MaybeUninit<SymbolTable<'static>> = MaybeUninit::uninit();
static mut STRING_TABLE: MaybeUninit<StringTable<'static>> = MaybeUninit::uninit();

unsafe fn get_symt() -> Option<&'static SymbolTable<'static>> {
    ELF_INITIALIZED.then(|| SYMBOL_TABLE.assume_init_ref())
}

pub fn init_symbols(sections: mb::ElfSectionsTag) {
    let mut symt = None;
    let mut strt = None;
    for sec in sections.sections() {
        match sec.name() {
            ".symtab" => symt = Some(sec),
            ".strtab" => strt = Some(sec),
            _ => (),
        }
    }
    if let (Some(symt), Some(strt)) = (symt, strt) {
        unsafe {
            ELF_INITIALIZED = true;
            STRING_TABLE = MaybeUninit::new(StringTable::new(strt.start_address() as _, strt.size() as _));
            SYMBOL_TABLE = MaybeUninit::new(SymbolTable::new(symt.start_address() as _, symt.size() as _, STRING_TABLE.assume_init_ref()));
        }
    }
}

pub fn find_symbol(addr: usize) -> Option<(&'static str, usize)> {
    let addr = addr as u64;
    if let Some(symt) = unsafe { get_symt() } {
        if let Some(sym) = symt.symbols().find(|s| (s.typ() == SymbolType::Func) && (addr >= s.addr()) && (addr < s.addr() + s.size())) {
            Some((sym.name(), (addr - sym.addr()) as usize))
        } else {
            None
        }
    } else {
        None
    }
}