mod acpi;
mod asm;
mod cmdline;
mod intr;
mod kernel;
mod log;
mod mpstart;
mod panic;
mod symbols;
mod timer;

use crate::{arch::x64::intr::apic, println};
use acpi::RSDT;
use chos_boot_defs::{virt, KernelBootInfo, KernelEntry};
use chos_x64::paging::PageTable;
use cmdline::iter_cmdline;

use spin::Barrier;

use multiboot2 as mb;

struct MpInfo {
    entry: KernelEntry,
    barrier: Barrier,
    kbi: KernelBootInfo,
    page_table: *mut PageTable,
}

#[no_mangle]
pub extern "C" fn boot_main(mbp: usize) -> ! {
    let mut logdev = log::Device::Serial;

    let mbh = unsafe { mb::load(mbp) };

    if let Some(cmdline) = mbh.command_line_tag() {
        for kv in iter_cmdline(cmdline.command_line()) {
            match kv {
                ("output", Some("serial")) => logdev = log::Device::Serial,
                ("output", Some("vga")) => logdev = log::Device::Vga,
                _ => (),
            }
        }
    }

    log::initialize(logdev);

    println!("############");
    println!("### BOOT ###");
    println!("############");

    if let Some(sections) = mbh.elf_sections_tag() {
        symbols::init_symbols(sections);
    }

    let rsdt = mbh
        .rsdp_v1_tag()
        .expect("No RSDT from Multiboot")
        .rsdt_address();
    let rsdt = unsafe { &*(rsdt as *const RSDT) };
    let madt = rsdt.madt().unwrap();
    let hpet = rsdt.hpet().unwrap();

    intr::initalize(madt);

    let kernel = if let Some(kernel) = mbh.module_tags().find(|m| {
        if let Some(("kernel", _)) = iter_cmdline(m.name()).next() {
            true
        } else {
            false
        }
    }) {
        let kernel = unsafe {
            core::slice::from_raw_parts(
                kernel.start_address() as *const u8,
                (kernel.end_address() - kernel.start_address()) as usize,
            )
        };
        chos_elf::Elf::new(kernel).expect("Invalid kernel ELF")
    } else {
        panic!("No kernel")
    };

    let memory_map = mbh.memory_map_tag().expect("Should have a memory map");
    println!("Memory map");
    for e in memory_map.all_memory_areas() {
        println!(
            "  {:012x}-{:012x} {:?}",
            e.start_address(),
            e.end_address(),
            e.typ()
        );
    }
    let mem_info = unsafe { kernel::map_kernel(&kernel, memory_map) };

    let entry = kernel.raw().entry + virt::KERNEL_CODE_BASE.as_u64();
    let entry: KernelEntry = unsafe { core::mem::transmute(entry) };

    let apic_count = madt.apic_count();

    let mp_info = MpInfo {
        entry,
        barrier: Barrier::new(apic_count),
        kbi: KernelBootInfo {
            elf: &kernel as *const _ as usize,
            multiboot_header: mbp,
            early_log: |args| println!("{}", args),
            mem_info,
        },
        page_table: unsafe { PageTable::get_current_page_table() },
    };

    timer::initialize(hpet);
    unsafe {
        mpstart::start_mp(
            madt,
            |id, mp_info| {
                let mp_info: &MpInfo = &*mp_info.cast();
                (*mp_info.page_table).set_page_table();
                mp_info.barrier.wait();
                (mp_info.entry)(&mp_info.kbi, id);
            },
            &mp_info as *const _ as _,
        )
    };
    mp_info.barrier.wait();

    entry(&mp_info.kbi, unsafe { apic().id() });
}
