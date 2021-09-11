use core::mem::MaybeUninit;
use core::slice::from_raw_parts;

use chos_lib::elf::{LookupStrategy, StrTab, Symtab, SymtabEntryType};
use multiboot2 as mb;

static mut ELF_INITIALIZED: bool = false;
static mut SYMBOL_TABLE: MaybeUninit<Symtab<'static>> = MaybeUninit::uninit();
static mut STRING_TABLE: MaybeUninit<StrTab<'static>> = MaybeUninit::uninit();

unsafe fn get_tables() -> Option<(&'static Symtab<'static>, &'static StrTab<'static>)> {
    ELF_INITIALIZED.then(|| {
        (
            SYMBOL_TABLE.assume_init_ref(),
            STRING_TABLE.assume_init_ref(),
        )
    })
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
            STRING_TABLE = MaybeUninit::new(StrTab::new(from_raw_parts(
                strt.start_address() as *const u8,
                strt.size() as usize,
            )));
            SYMBOL_TABLE = MaybeUninit::new(Symtab::new(
                from_raw_parts(symt.start_address() as *const u8, symt.size() as usize),
                LookupStrategy::Linear,
            ));
        }
    }
}

pub fn find_symbol(addr: usize) -> Option<(&'static str, usize)> {
    let addr = addr as u64;
    if let Some((symt, strt)) = unsafe { get_tables() } {
        if let Some(sym) = symt.iter().find(|s| {
            (s.typ() == SymtabEntryType::Func)
                && (addr >= s.value())
                && (addr < s.value() + s.size())
        }) {
            Some((sym.name(strt).unwrap_or(""), (addr - sym.value()) as usize))
        } else {
            None
        }
    } else {
        None
    }
}
