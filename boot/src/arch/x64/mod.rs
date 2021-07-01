mod acpi;
mod asm;
mod cmdline;
mod intr;
mod log;
mod kernel;
mod mpstart;
mod panic;
mod symbols;
mod timer;

use core::sync::atomic::AtomicUsize;

use crate::{arch::x64::intr::apic, println};
use acpi::RSDT;
use chos_boot_defs::{KernelBootInfo, KernelEntry, virt};
use cmdline::iter_cmdline;

use spin::Barrier;

use multiboot2 as mb;

struct MpInfo {
    entry: KernelEntry,
    barrier: Barrier,
    kbi: KernelBootInfo,
    page_table_phys: usize,
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

    if let Some(sections) = mbh.elf_sections_tag() {
        symbols::init_symbols(sections);
    }

    let rsdt = mbh.rsdp_v1_tag().unwrap().rsdt_address();
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
        let kernel = unsafe { core::slice::from_raw_parts(
            kernel.start_address() as *const u8,
            (kernel.end_address() - kernel.start_address()) as usize)
        };
        chos_elf::Elf::new(kernel).expect("Invalid kernel ELF")
    } else {
        panic!("No kernel")
    };

    unsafe { kernel::map_kernel(&kernel) };

    let entry = kernel.raw().entry + virt::KERNEL_CODE_BASE;
    let entry: KernelEntry = unsafe { core::mem::transmute(entry) };

    let apic_count = madt.apic_count();

    let mp_info = MpInfo {
        entry,
        barrier: Barrier::new(apic_count),
        kbi: KernelBootInfo {
            elf: &kernel as *const _ as usize,
            multiboot_header: mbp,
            early_log: |args| println!("{}", args),
        },
        page_table_phys: unsafe {
            let cr3: usize;
            asm! {
                "mov %cr3, {}",
                lateout(reg) cr3,
                options(att_syntax, nomem, nostack),
            };
            cr3
        }
    };

    timer::initialize(hpet);
    let n = unsafe { mpstart::start_mp(
        madt,
        |id, mp_info| {
            println!("Here {}", id);
            let mp_info: &MpInfo = &*mp_info.cast();
            let cr3: usize;
            asm! {
                "mov {}, %cr3",
                in(reg) mp_info.page_table_phys,
                options(att_syntax, nomem, nostack),
            };
            mp_info.barrier.wait();
            (mp_info.entry)(&mp_info.kbi, id);
        },
        &mp_info as *const _ as _,
    ) };
    mp_info.barrier.wait();

    entry(&mp_info.kbi, unsafe { apic().id() });
}
