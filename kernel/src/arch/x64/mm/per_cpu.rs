use alloc::boxed::Box;
use core::intrinsics::copy_nonoverlapping;
use core::mem::MaybeUninit;
use core::ptr::write_bytes;

use chos_config::arch::mm::virt;
use chos_lib::arch::mm::FrameSize4K;
use chos_lib::arch::regs::FS;
use chos_lib::elf::{Elf, ProgramEntryType};
use chos_lib::int::{log2u64, CeilDiv};
use chos_lib::log::debug;
use chos_lib::mm::{FrameSize, MapFlags, MapperFlush, PAddr, PFrameRange, RangeMapper, VAddr};

use super::virt::MMFrameAllocator;
use crate::mm::phys::{raw_alloc, AllocFlags};
use crate::mm::{per_cpu, CpuInfo, PerCpu};

#[repr(C)]
#[derive(Debug)]
struct TlsIndex {
    module: u64,
    offset: u64,
}

#[repr(C)]
#[derive(Debug)]
struct TlsData {
    kernel_tls_end: VAddr,
    kernel_tls_size: u64,
    id: u64,
    phys_tls_base: PAddr,
    pages: u64,
}
static mut TLS_DATA: MaybeUninit<&'static [TlsData]> = MaybeUninit::uninit();

pub fn per_cpu_data() -> VAddr {
    FS::read()
}

pub fn per_cpu_base_for(id: usize) -> VAddr {
    let tls_data = unsafe { &TLS_DATA.assume_init_ref()[id] };
    tls_data.kernel_tls_end
}

#[no_mangle]
unsafe extern "C" fn __tls_get_addr(idx: &TlsIndex) -> *mut () {
    let tls_data = per_cpu_data().as_ref::<TlsData>();
    let addr = match idx.module {
        0 => (tls_data.kernel_tls_end - tls_data.kernel_tls_size + idx.offset).as_mut_ptr(),
        _ => unimplemented!(
            "Module Per Cpu data not implemented: __tls_get_addr({:?})",
            idx
        ),
    };
    addr
}

pub unsafe fn init_per_cpu_data(
    core_count: usize,
    elf: &Elf,
    mapper: &mut impl RangeMapper<FrameSize4K, PGTFrameSize = FrameSize4K>,
) {
    let tls_entries = elf
        .program()
        .iter()
        .filter(|e| e.typ() == ProgramEntryType::Tls);
    if tls_entries.clone().count() == 0 {
        TLS_DATA = MaybeUninit::new(&[]);
        return;
    }
    let total_size: u64 = tls_entries.clone().map(|e| e.mem_size()).sum();
    assert_eq!(
        tls_entries.clone().count(),
        1,
        "Only supporting 1 TLS program header entry"
    );
    let total_pages = total_size.ceil_div(FrameSize4K::PAGE_SIZE);
    let vbase = virt::PER_CPU_BASE;
    let mut vcur = vbase;
    let mut pbases = Box::new_uninit_slice(core_count);
    for i in 0..core_count {
        let mut remaining = total_pages;
        while remaining > 0 {
            let order = log2u64(remaining);
            let pages = raw_alloc::alloc_pages(order as u8, AllocFlags::empty())
                .expect("Alloc should not fail");
            mapper
                .map_range(
                    PFrameRange::new(pages, pages.add(1 << order)),
                    vcur,
                    MapFlags::WRITE | MapFlags::GLOBAL,
                    &mut MMFrameAllocator,
                )
                .expect("Alloc should not fail")
                .flush();
            remaining -= 1 << order;
            vcur = vcur.add(1 << order);

            pbases[i] = MaybeUninit::new(pages);
        }
    }

    let pbases = pbases.assume_init();

    for tls in tls_entries {
        for i in 0..core_count {
            let addr = vbase.add((i as u64) * total_pages).addr().as_mut_ptr();
            copy_nonoverlapping(
                elf.get_buffer(tls.offset() as usize, tls.file_size() as usize)
                    .as_ptr(),
                addr,
                tls.file_size() as usize,
            );
            if tls.mem_size() > tls.file_size() {
                let addr = addr.add(tls.file_size() as usize);
                write_bytes(addr, 0, (tls.mem_size() - tls.file_size()) as usize);
            }
        }
    }

    let mut tls_data = Box::new_uninit_slice(core_count as usize);
    for i in 0..core_count {
        tls_data[i as usize] = MaybeUninit::new(TlsData {
            kernel_tls_end: vbase.add((i as u64) * total_pages).addr() + total_pages * FrameSize4K::PAGE_SIZE,
            kernel_tls_size: total_pages * FrameSize4K::PAGE_SIZE,
            id: i as u64,
            pages: total_pages,
            phys_tls_base: pbases[i].addr(),
        });
    }
    let tls_data = tls_data.assume_init();

    for entry in tls_data.iter() {
        debug!(
            "TLS [{}] -> {:#x}",
            entry.id,
            entry.kernel_tls_end,
        );
    }
    TLS_DATA = MaybeUninit::new(Box::leak(tls_data));
    init_per_cpu_data_for_cpu(0);
}

per_cpu! {
    pub static mut ref CPU_INFO: CpuInfo = CpuInfo {
        id: usize::MAX,
    };
}

pub unsafe fn init_per_cpu_data_for_cpu(core_id: usize) {
    let tls_data = *TLS_DATA.assume_init_ref();
    // Check that we have per-cpu data
    if tls_data.len() != 0 {
        FS::write((&TLS_DATA.assume_init_ref()[core_id]).into())
    }
    CPU_INFO.with_static(|info| {
        *info = CpuInfo { id: core_id };
    });
}

pub fn arch_this_cpu_info() -> &'static CpuInfo {
    unsafe { CPU_INFO.get_ref() }
}
