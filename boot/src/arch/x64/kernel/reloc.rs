use core::mem::transmute;
use core::ptr::write_volatile;

use chos_elf::{Elf, Rela, RelaEntry, Symtab, SymtabEntry, SymtabEntryType};

use chos_boot_defs::virt::KERNEL_CODE_BASE as BASE;

fn check_symbol(sym: SymtabEntry) {
    assert!(sym.value() != 0 || sym.typ() != SymtabEntryType::NoType, "Symbol [{}] is not defined", unsafe { *core::ptr::addr_of!(sym.raw().name) });
}

fn symbol_value(sym: SymtabEntry) -> i64 {
    check_symbol(sym);
    unsafe { transmute(sym.value()) }
}

fn symbol_offset_value(sym: SymtabEntry) -> i64 {
    check_symbol(sym);
    unsafe { transmute(sym.value() + BASE.as_u64()) }
}

pub unsafe fn do_relocation(symtab: &Symtab, e: &RelaEntry) {
    use chos_elf::X64RelaType::*;
    // crate::println!("Reloc {:?} @ {:08x} + {:08x}", e.x64_typ(), e.offset(), e.addend());
    let off = (e.offset() + BASE.as_u64()) as *mut i64;
    match e.x64_typ() {
        None => (),
        _64 => write_volatile(
            off,
            symbol_offset_value(symtab.get(e.sym() as usize)) + e.addend(),
        ),
        GlobDat | JumpSlot => write_volatile(
            off,
            symbol_offset_value(symtab.get(e.sym() as usize)),
        ),
        Relative => write_volatile(
            off,
            transmute::<_, i64>(BASE) + e.addend(),
        ),
        DtpMod64 => write_volatile(off, 0),
        DtpOff64 => write_volatile(
            off,
            symbol_value(symtab.get(e.sym() as usize)),
        ),
        _ => todo!("Implement relocation type {:x?}", e.x64_typ()),
    }
}

unsafe fn apply_rela(symtab: &Symtab, rela: &Rela) {
    for e in rela {
        do_relocation(&symtab, &e);
    }
}

pub unsafe fn apply_relocations(elf: &Elf) {
    if let Some(dyna) = elf.program().dynamic(elf) {
        if let Some(symtab) = dyna.symtab(elf) {
            if let Some(rela) = dyna.relaplt(elf)  {
                apply_rela(&symtab, &rela);
            }
            if let Some(rela) = dyna.rela(elf) {
                apply_rela(&symtab, &rela);
            }
        }
    }
}