mod asm;
mod cmdline;
mod intr;
mod kernel;
mod log;
mod mpstart;
mod panic;
mod symbols;
mod timer;

use chos_config::arch::mm::virt;
use chos_lib::arch::boot::ArchKernelBootInfo;
use chos_lib::arch::x64::acpi::Rsdt;
use chos_lib::arch::x64::mm::PageTable;
use chos_lib::boot::{KernelBootInfo, KernelEntry};
use chos_lib::log::{debug, println, Bytes};
use chos_lib::sync::spin::barrier::Barrier;
use cmdline::iter_cmdline;
use multiboot2 as mb;

use crate::arch::x64::intr::apic;

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

    println!(
        r#"
   .oooooo.   ooooo   ooooo   .oooooo.    .oooooo..o 
  d8P'  `Y8b  `888'   `888'  d8P'  `Y8b  d8P'    `Y8 
 888           888     888  888      888 Y88bo.      
 888           888ooooo888  888      888  `"Y8888o.  
 888           888     888  888      888      `"Y88b 
 `88b    ooo   888     888  `88b    d88' oo     .d8P 
  `Y8bood8P'  o888o   o888o  `Y8bood8P'  8""88888P'  
"#
    );

    debug!("############");
    debug!("### BOOT ###");
    debug!("############");

    if let Some(sections) = mbh.elf_sections_tag() {
        symbols::init_symbols(sections);
    }

    let rsdt = mbh
        .rsdp_v1_tag()
        .expect("No RSDT from Multiboot")
        .rsdt_address();
    let rsdt = unsafe { &*(rsdt as *const Rsdt) };
    let madt = rsdt.madt().unwrap();
    let hpet = rsdt.hpet().unwrap();

    intr::initalize(madt);

    let kernel = if let Some(kernel) = mbh
        .module_tags()
        .find(|m| matches!(iter_cmdline(m.name()).next(), Some(("kernel", _))))
    {
        let kernel = unsafe {
            core::slice::from_raw_parts(
                kernel.start_address() as *const u8,
                (kernel.end_address() - kernel.start_address()) as usize,
            )
        };
        chos_lib::elf::Elf::new(kernel).expect("Invalid kernel ELF")
    } else {
        panic!("No kernel")
    };

    let memory_map = mbh.memory_map_tag().expect("Should have a memory map");
    debug!("Memory map");
    let mut total_mem = 0;
    for e in memory_map.all_memory_areas() {
        debug!(
            "  {:012x}-{:012x} {:?} {}",
            e.start_address(),
            e.end_address(),
            e.typ(),
            Bytes(e.end_address() - e.start_address()),
        );
        if let mb::MemoryAreaType::Available = e.typ() {
            total_mem += e.end_address() - e.start_address();
        }
    }
    debug!("Total available memory: {}", Bytes(total_mem));
    let mem_info = unsafe { kernel::map_kernel(&kernel, memory_map) };

    let entry = kernel.raw().entry + virt::STATIC_BASE.as_u64();
    let entry: KernelEntry = unsafe { core::mem::transmute(entry) };

    let apic_count = madt.apic_count();

    let mp_info = MpInfo {
        entry,
        barrier: Barrier::new(apic_count),
        kbi: KernelBootInfo {
            elf: &kernel as *const _ as usize,
            multiboot_header: mbp,
            early_log: &log::BOOT_LOG_HANDLER,
            mem_info,
            arch: ArchKernelBootInfo { rsdt },
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
