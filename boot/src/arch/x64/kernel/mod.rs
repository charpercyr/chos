
mod mapper;
mod palloc;
mod reloc;

use crate::arch::x64::kernel::mapper::{Mapper};
use crate::arch::x64::kernel::palloc::PAlloc;
use crate::arch::x64::kernel::reloc::apply_relocations;
use crate::println;

use core::ptr::{copy_nonoverlapping, write_bytes};
use core::str::from_utf8_unchecked;
use core::u8;

use chos_boot_defs::{phys, virt};

use chos_lib::int::CeilDiv;
use chos_lib::iter::IteratorExt;

use chos_elf::{Elf, ProgramEntryFlags, ProgramEntryType};
use chos_x64::paging::{PAGE_SIZE, PageTable};

unsafe fn use_page_table(tbl: &mut PageTable) {
    asm! {
        "mov %rax, %cr3",
        in("rax") tbl,
        options(att_syntax, nostack, nomem),
    }
}

pub unsafe fn map_kernel(kernel: &Elf) {
    let iter = kernel
        .program()
        .iter()
        .filter(|p| p.typ() == ProgramEntryType::Load);
    let (pmap_start, pmap_end) = iter
        .clone()
        .map(|p| {
            (
                p.vaddr() / p.align() * p.align(),
                (p.vaddr() + p.mem_size()).ceil_div(p.align()) * p.align(),
            )
        })
        .min_max()
        .expect("No LOAD program entries");
    let (pmap_start, pmap_end) = (
        pmap_start + phys::KERNEL_DATA_BASE,
        pmap_end + phys::KERNEL_DATA_BASE,
    );

    println!("ELF @ {:08x}", kernel.raw() as *const _ as usize);
    println!("INIT {:08x} - {:08x}", pmap_start, pmap_end);
    write_bytes(
        pmap_start as *mut u8,
        0xcc,
        (pmap_end - pmap_start) as usize,
    );

    for p in iter.clone() {
        let data = kernel.get_buffer(p.offset() as usize, p.file_size() as usize);
        println!(
            "COPY {:08x} - {:08x} to {:08x} - {:08x}",
            data.as_ptr() as u64,
            data.as_ptr() as u64 + p.file_size(),
            phys::KERNEL_DATA_BASE + p.vaddr(),
            phys::KERNEL_DATA_BASE + p.vaddr() + p.file_size()
        );
        copy_nonoverlapping(data.as_ptr(), (phys::KERNEL_DATA_BASE + p.vaddr()) as *mut u8, p.file_size() as usize);
    }

    let mut palloc = PAlloc::new(pmap_end as *mut u8);
    let mut mapper = Mapper::new(&mut palloc);
    mapper.identity_map_4g(&mut palloc);

    for p in iter {
        assert_eq!(p.align() as usize, PAGE_SIZE);
        let pstart = (phys::KERNEL_DATA_BASE + p.vaddr()) / p.align() * p.align();
        let pend = (phys::KERNEL_DATA_BASE + p.vaddr() + p.mem_size()).ceil_div(p.align()) * p.align();
        let vstart = (virt::KERNEL_CODE_BASE + p.vaddr()) / p.align() * p.align();
        let vend = (virt::KERNEL_CODE_BASE + p.vaddr() + p.mem_size()).ceil_div(p.align()) * p.align();
        let mut perms = [b'-'; 3];
        let flags = p.flags();
        if flags.contains(ProgramEntryFlags::READ) {
            perms[0] = b'r';
        }
        if flags.contains(ProgramEntryFlags::WRITE) {
            perms[1] = b'w';
        }
        if flags.contains(ProgramEntryFlags::EXEC) {
            perms[2] = b'x';
        }
        let perms = from_utf8_unchecked(&perms);
        println!("MAP {:08x} - {:08x} to {:016x} - {:016x} {}", pstart, pend, vstart, vend, perms);
        let pages = (pend - pstart) / p.align();
        for i in 0..pages {
            let paddr = pstart + i * PAGE_SIZE as u64;
            let vaddr = vstart + i * PAGE_SIZE as u64;
            mapper.map(paddr, vaddr, flags.contains(ProgramEntryFlags::WRITE), flags.contains(ProgramEntryFlags::EXEC), &mut palloc);
        }
    }
    palloc.map_self(virt::KERNEL_PT_BASE, &mut mapper);

    use_page_table(mapper.p4);

    apply_relocations(kernel);
}
