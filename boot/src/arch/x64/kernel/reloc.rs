use core::mem::transmute;
use core::ptr::write_volatile;

use chos_config::arch::mm::virt::STATIC_BASE as BASE;
use chos_lib::elf::{Elf, Rela, RelaEntry, StrTab, Symtab, SymtabEntry, SymtabEntryType};

fn check_symbol(idx: usize, sym: SymtabEntry, strtab: Option<&StrTab>) {
    if let Some(name) = strtab.and_then(|strtab| sym.name(strtab)) {
        assert!(
            sym.value() != 0 || sym.typ() != SymtabEntryType::NoType,
            "Symbol '{}' [{}] is not defined",
            name,
            idx,
        );
    } else {
        assert!(
            sym.value() != 0 || sym.typ() != SymtabEntryType::NoType,
            "Symbol [{}] is not defined",
            idx,
        );
    }
}

fn symbol_value(idx: usize, sym: SymtabEntry, strtab: Option<&StrTab>) -> i64 {
    check_symbol(idx, sym, strtab);
    unsafe { transmute(sym.value()) }
}

fn symbol_offset_value(idx: usize, sym: SymtabEntry, strtab: Option<&StrTab>) -> i64 {
    check_symbol(idx, sym, strtab);
    unsafe { transmute(sym.value() + BASE.as_u64()) }
}

pub unsafe fn do_relocation(symtab: &Symtab, e: &RelaEntry, strtab: Option<&StrTab>) {
    use chos_lib::elf::X64RelaType::*;
    let off = (e.offset() + BASE.as_u64()) as *mut i64;
    match e.x64_typ() {
        None => (),
        _64 => write_volatile(
            off,
            symbol_offset_value(e.sym() as usize, symtab.get(e.sym() as usize), strtab)
                + e.addend(),
        ),
        GlobDat | JumpSlot => write_volatile(
            off,
            symbol_offset_value(e.sym() as usize, symtab.get(e.sym() as usize), strtab),
        ),
        Relative => write_volatile(off, transmute::<_, i64>(BASE) + e.addend()),
        DtpMod64 => write_volatile(off, 0),
        DtpOff64 => write_volatile(
            off,
            symbol_value(e.sym() as usize, symtab.get(e.sym() as usize), strtab),
        ),
        _ => todo!("Implement relocation type {:x?}", e.x64_typ()),
    }
}

unsafe fn apply_rela(symtab: &Symtab, rela: &Rela, strtab: Option<&StrTab>) {
    for e in rela.iter() {
        do_relocation(symtab, &e, strtab);
    }
}

pub unsafe fn apply_relocations(elf: &Elf) {
    if let Some(dyna) = elf.program().dynamic(elf) {
        let strtab = dyna.strtab(elf);
        if let Some(symtab) = dyna.symtab(elf) {
            if let Some(rela) = dyna.relaplt(elf) {
                apply_rela(&symtab, &rela, strtab.as_ref());
            }
            if let Some(rela) = dyna.rela(elf) {
                apply_rela(&symtab, &rela, strtab.as_ref());
            }
        }
    }
}
