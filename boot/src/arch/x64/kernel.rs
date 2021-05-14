use core::ptr::{copy_nonoverlapping, write};

use chos_boot_defs::{phys, virt, BootMemoryMap};

use chos_elf::{Elf64, ProgramEntryType};

use chos_x64::paging::{PageEntry, PageTable, make_canonical, split_virtual_address};

// Maps the kernel in the following order, aligned to pages
// Kernel code
// Page Table
// Memory Map

const PAGE_SIZE: usize = 4096;

pub struct KernelMap {
    pub base: usize,
    pub page_table_off: usize,
    pub boot_memory_map_off: usize,
}

fn create_new_page(addr: u64, write: bool, exec: bool) -> PageEntry {
    let mut entry = PageEntry::zero();
    entry
        .set_phys_addr(addr)
        .set_writable(write)
        .set_no_execute(!exec)
        .set_global(true)
        .set_present(true)
        .set_no_cache(true)
        .set_write_through(true)
    ;
    entry
}

unsafe fn alloc_page_table<'a>(mut data: *mut u8) -> (*mut u8, &'a mut PageTable) {
    let pt: *mut PageTable = data.cast();
    write(pt, PageTable::empty());
    let pt = &mut *pt;
    data = data.add(4096);
    (data, pt)
}

type MapPage<'a> = &'a mut dyn FnMut(u64, bool, bool);

unsafe fn copy_kernel_elf(kernel: &Elf64) -> (*mut u8, usize) {
    let mut data = phys::KERNEL_DATA_BASE as *mut u8;
    let mut start_data = data;

    let mut total_size = 0usize;
    for p in kernel.program() {
        if p.typ() == ProgramEntryType::Load {
            let base_addr = data.add(p.vaddr() as usize);
            super::println!("Copying code to {:p} + {:#x} {:?}", base_addr, p.file_size(), p.flags());
            copy_nonoverlapping(
                kernel.data_ptr(p.offset() as usize),
                base_addr,
                p.file_size() as usize,
            );
            total_size = usize::max(total_size, (p.vaddr() + p.mem_size()) as usize);
        }
    }

    data = data.add(total_size);
    data = data.add(data.align_offset(PAGE_SIZE));

    (data, data.offset_from(start_data) as usize)
}

struct PageMapper {
    p4: *mut PageTable,
    indices: [u16; 4],
}

unsafe fn map_kernel_elf(kernel: &Elf64, map_page: MapPage<'_>) {
    map_page(0x0000, false, true);
    map_page(0x1000, true, false);
    map_page(0x2000, false, false);
}

pub unsafe fn map_kernel(kernel: &Elf64) -> KernelMap {
    let (data, len) = copy_kernel_elf(kernel);

    let page_table_off = data.offset_from(phys::KERNEL_DATA_BASE as *mut u8) as usize;

    let (data, p4) = alloc_page_table(data);
    let (data, p3) = alloc_page_table(data);
    let (data, p2) = alloc_page_table(data);
    let (mut data, mut p1) = alloc_page_table(data);

    p4[128] = create_new_page(p3 as *mut PageTable as u64, true, false);
    p3[0] = create_new_page(p2 as *mut PageTable as u64, true, false);
    p2[0] = create_new_page(p1 as *mut PageTable as u64, true, false);

    let mut map_page = {
        let (mut p4_i, mut p3_i, mut p2_i, mut p1_i, off) =
            split_virtual_address(virt::KERNEL_CODE_BASE as u64).unwrap();
        assert_eq!(off, 0);
        move |addr: u64, write: bool, exec: bool| {
            let mut page = PageEntry::zero();
            page.set_phys_addr(addr)
                .set_writable(write)
                .set_no_execute(!exec)
                .set_present(true);
            let vaddr = p4_i as u64 * 512 * 512 * 512 * 4096
                + p3_i as u64 * 512 * 512 * 4096
                + p2_i as u64 * 512 * 4096
                + p1_i as u64 * 4096;
            let vaddr = make_canonical(vaddr);
            crate::println!(
                "Mapping 0x{:04x} @ {} - {} - {} - {} (0x{:016x})",
                addr,
                p4_i,
                p3_i,
                p2_i,
                p1_i,
                vaddr,
            );
            p1_i += 1;
        }
    };

    map_kernel_elf(kernel, &mut map_page);

    KernelMap {
        base: phys::KERNEL_DATA_BASE,
        page_table_off,
        boot_memory_map_off: 0,
    }
}
