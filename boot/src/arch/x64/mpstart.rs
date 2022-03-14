use core::mem::MaybeUninit;
use core::ptr::null;

use chos_lib::arch::acpi::madt;
use chos_lib::arch::apic::Apic;
use chos_lib::arch::mm::{VAddr, PAGE_SIZE64};
use chos_lib::arch::qemu::{exit_qemu, QemuStatus};
use chos_lib::arch::regs::CR3;
use chos_lib::log::println;
use chos_lib::mm::VFrame;
use chos_lib::sync::SpinBarrier;
use uefi::table::boot::{MemoryDescriptor, MemoryType};

use super::intr::initialize_secondary;

const MPSTART_RELOC_ADDRESS: VAddr = VAddr::new(0x8000);

extern "C" {
    static MPSTART_16: u8;
    static MPSTART_16_LEN: usize;
}

type MpStartFn = fn(usize, *const ()) -> !;

static mut MPSTART_FN: MaybeUninit<MpStartFn> = MaybeUninit::uninit();
static mut MPSTART_USER: *const () = null();
static mut MPSTART_APIC_BASE: VAddr = VAddr::null();
static mut MPSTART_BARRIER: MaybeUninit<SpinBarrier> = MaybeUninit::uninit();

#[no_mangle]
static mut MPSTART_PDT4: u64 = 0;
#[no_mangle]
static MPSTART_STACK_BASE: u64 = 0x9000;
#[no_mangle]
static MPSTART_STACK_STRIDE: u64 = 0x4000;
unsafe fn start_processor(apic: &mut Apic, lapic_id: u8) {
    let mpstart_page = VFrame::new(MPSTART_RELOC_ADDRESS);
    apic.commands()
        .start_ap(lapic_id, mpstart_page, |d| super::timer::delay(d).unwrap());
}

fn lapics<'a>(madt: &'a madt::Madt) -> impl Iterator<Item = &'a madt::LAPICEntry> {
    madt.entries().filter_map(madt::Entry::lapic)
}

pub fn is_mp_ready<'a>(
    memory_map: impl Iterator<Item = &'a MemoryDescriptor>,
    madt: &madt::Madt,
) -> bool {
    let apic_count = madt.apic_count();
    let mut mpstart_good = false;
    let mut stack_good = false;
    for e in memory_map {
        println!("{:?}", e);
        let range_start = e.phys_start;
        let range_end = e.phys_start + e.page_count * PAGE_SIZE64;
        if MPSTART_RELOC_ADDRESS.as_u64() >= range_start
            && MPSTART_RELOC_ADDRESS.as_u64() + (unsafe { MPSTART_16_LEN } as u64) < range_end
        {
            if e.ty == MemoryType::CONVENTIONAL {
                mpstart_good = true;
            }
        }
        if MPSTART_STACK_BASE >= range_start
            && MPSTART_STACK_BASE + (MPSTART_STACK_STRIDE * apic_count as u64) < range_end
        {
            stack_good = true;
        }
        if mpstart_good && stack_good {
            return true;
        }
    }
    mpstart_good && stack_good
}

pub unsafe fn start_mp(madt: &madt::Madt, start_fn: MpStartFn, user: *const ()) -> usize {
    core::ptr::copy_nonoverlapping(
        &MPSTART_16,
        MPSTART_RELOC_ADDRESS.as_mut_ptr(),
        MPSTART_16_LEN,
    );

    let apic = super::intr::apic();
    let this_apic_id = apic.id();

    let apic_ids = lapics(madt).map(|e| e.apic_id);
    let count = madt.apic_count();

    MPSTART_FN = MaybeUninit::new(start_fn);
    MPSTART_USER = user;
    MPSTART_APIC_BASE = VAddr::new_unchecked(madt.lapic_address as u64);
    MPSTART_BARRIER = MaybeUninit::new(SpinBarrier::new(count));

    MPSTART_PDT4 = CR3.read_raw();

    for lapic_id in apic_ids {
        if lapic_id != this_apic_id as u8 {
            start_processor(apic, lapic_id);
        }
    }

    MPSTART_BARRIER.assume_init_ref().wait();

    count
}

#[no_mangle]
fn secondary_main() -> ! {
    initialize_secondary();
    exit_qemu(QemuStatus::Error);
}
