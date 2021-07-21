
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

use chos_boot_defs::{KernelMemEntry, KernelMemInfo, phys, virt};

use chos_lib::int::CeilDiv;
use chos_lib::iter::IteratorExt;

use chos_elf::{Elf, ProgramEntryFlags, ProgramEntryType};
use chos_x64::paging::{PAddr, PAGE_SIZE, VAddr};
use multiboot2::MemoryMapTag;

pub unsafe fn map_kernel(kernel: &Elf, memory: &MemoryMapTag) -> KernelMemInfo {
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
        pmap_start + phys::KERNEL_DATA_BASE.as_u64(),
        pmap_end + phys::KERNEL_DATA_BASE.as_u64(),
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
            phys::KERNEL_DATA_BASE.as_u64() + p.vaddr(),
            phys::KERNEL_DATA_BASE.as_u64() + p.vaddr() + p.file_size()
        );
        copy_nonoverlapping(data.as_ptr(), (phys::KERNEL_DATA_BASE.as_u64() + p.vaddr()) as *mut u8, p.file_size() as usize);
        if p.file_size() < p.mem_size() {
            println!(
                "ZERO {:08x} - {:08x}",
                phys::KERNEL_DATA_BASE.as_u64() + p.vaddr() + p.file_size(),
                phys::KERNEL_DATA_BASE.as_u64() + p.vaddr() + p.mem_size(),

            );
            write_bytes((phys::KERNEL_DATA_BASE.as_u64() + p.vaddr() + p.file_size()) as *mut u8, 0, (p.mem_size() - p.file_size()) as usize);
        }
    }

    let mut palloc = PAlloc::new(pmap_end as *mut u8);
    let mut mapper = Mapper::new(&mut palloc);
    mapper.identity_map_memory(&mut palloc, memory);

    for p in iter {
        assert_eq!(p.align() as usize, PAGE_SIZE);
        let pstart = (phys::KERNEL_DATA_BASE.as_u64() + p.vaddr()) / p.align() * p.align();
        let pend = (phys::KERNEL_DATA_BASE.as_u64() + p.vaddr() + p.mem_size()).ceil_div(p.align()) * p.align();
        let vstart = (virt::KERNEL_CODE_BASE.as_u64() + p.vaddr()) / p.align() * p.align();
        let vend = (virt::KERNEL_CODE_BASE.as_u64() + p.vaddr() + p.mem_size()).ceil_div(p.align()) * p.align();
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
            mapper.map(PAddr::new(paddr), VAddr::new(vaddr).unwrap(), flags.contains(ProgramEntryFlags::WRITE), flags.contains(ProgramEntryFlags::EXEC), &mut palloc);
        }
    }
    palloc.map_self(virt::KERNEL_PT_BASE, &mut mapper);

    mapper.p4.set_page_table();

    apply_relocations(kernel);

    KernelMemInfo {
        code: KernelMemEntry {
            phys: phys::KERNEL_DATA_BASE,
            virt: virt::KERNEL_CODE_BASE,
            size: (pmap_end - pmap_start) as usize,
        },
        pt: KernelMemEntry {
            phys: phys::KERNEL_DATA_BASE.add(pmap_end).sub(pmap_start),
            virt: virt::KERNEL_PT_BASE,
            size: palloc.total_size(),
        }
    }
}
