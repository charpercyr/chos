mod mapper;
mod palloc;
mod reloc;

use core::ptr::{copy_nonoverlapping, write_bytes};
use core::u8;

use chos_config::arch::x64::mm::{phys, virt};
use chos_lib::arch::mm::PageTable;
use chos_lib::boot::{KernelMemEntry, KernelMemInfo};
use chos_lib::elf::{Elf, ProgramEntryType};
use chos_lib::fmt::Bytes;
use chos_lib::int::CeilDiv;
use chos_lib::log::debug;
use chos_lib::mm::{MapFlags, MapperFlush, PAddr, PFrame, RangeMapper, VAddr, VFrame};
use multiboot2::MemoryMapTag;

use crate::arch::x64::kernel::mapper::BootMapper;
use crate::arch::x64::kernel::palloc::PAlloc;
use crate::arch::x64::kernel::reloc::apply_relocations;

pub unsafe fn map_kernel(kernel: &Elf, memory: &MemoryMapTag) -> KernelMemInfo {
    let iter = kernel
        .program()
        .iter()
        .filter(|p| p.typ() == ProgramEntryType::Load);
    let (pmap_start, pmap_end) = iter.clone().fold((u64::MAX, u64::MIN), |(min, max), p| {
        (
            u64::min(min, p.vaddr() / p.align() * p.align()),
            u64::max(
                max,
                (p.vaddr() + p.mem_size()).ceil_div(p.align()) * p.align(),
            ),
        )
    });
    let (pmap_start, pmap_end) = (
        pmap_start + phys::KERNEL_DATA_BASE.addr().as_u64(),
        pmap_end + phys::KERNEL_DATA_BASE.addr().as_u64(),
    );

    debug!(
        "ELF @ {:#08x} - {:#08x} {}",
        kernel.raw() as *const _ as usize,
        kernel.raw() as *const _ as usize + kernel.data().len(),
        Bytes(kernel.data().len() as u64)
    );
    debug!("INIT {:#08x} - {:#08x}", pmap_start, pmap_end);
    write_bytes(
        pmap_start as *mut u8,
        0xcc,
        (pmap_end - pmap_start) as usize,
    );

    for p in iter.clone() {
        let data = kernel.get_buffer(p.offset() as usize, p.file_size() as usize);
        debug!(
            "COPY {:08x} - {:08x} to {:08x} - {:08x}",
            data.as_ptr() as u64,
            data.as_ptr() as u64 + p.file_size(),
            phys::KERNEL_DATA_BASE.addr().as_u64() + p.vaddr(),
            phys::KERNEL_DATA_BASE.addr().as_u64() + p.vaddr() + p.file_size()
        );
        copy_nonoverlapping(
            data.as_ptr(),
            (phys::KERNEL_DATA_BASE.addr().as_u64() + p.vaddr()) as *mut u8,
            p.file_size() as usize,
        );
        if p.file_size() < p.mem_size() {
            debug!(
                "ZERO {:08x} - {:08x}",
                phys::KERNEL_DATA_BASE.addr().as_u64() + p.vaddr() + p.file_size(),
                phys::KERNEL_DATA_BASE.addr().as_u64() + p.vaddr() + p.mem_size(),
            );
            write_bytes(
                (phys::KERNEL_DATA_BASE.addr().as_u64() + p.vaddr() + p.file_size()) as *mut u8,
                0,
                (p.mem_size() - p.file_size()) as usize,
            );
        }
    }

    let mut palloc = PAlloc::new(PFrame::new_align_up(PAddr::new(pmap_end)));
    let mut mapper = BootMapper::new(&mut palloc);
    mapper.identity_map_memory(&mut palloc, memory, VFrame::new_unchecked(VAddr::null()));
    mapper.identity_map_memory(
        &mut palloc,
        memory,
        VFrame::new_unchecked(virt::PHYSICAL_MAP_BASE.addr()),
    );
    mapper
        .mapper
        .map_elf_load_sections(
            kernel,
            phys::KERNEL_DATA_BASE,
            virt::STATIC_BASE,
            MapFlags::empty(),
            &mut palloc,
        )
        .expect("Mapping ELF failed")
        .ignore();
    PageTable::set_page_table(PFrame::new_unchecked(PAddr::new(
        mapper.mapper.inner_mut().p4 as *mut _ as u64,
    )));

    apply_relocations(kernel);

    KernelMemInfo {
        code: KernelMemEntry {
            phys: phys::KERNEL_DATA_BASE.addr(),
            size: (pmap_end - pmap_start) as usize,
        },
        total_size: memory
            .all_memory_areas()
            .map(|e| e.end_address())
            .max()
            .expect("Memory map is empty"),
    }
}
