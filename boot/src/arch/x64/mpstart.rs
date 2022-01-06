use core::mem::MaybeUninit;
use core::ptr::null;

use chos_lib::arch::mm::PAGE_SHIFT;
use chos_lib::arch::regs::Cr3;
use chos_lib::arch::x64::acpi::madt;
use chos_lib::arch::x64::apic::Apic;
use chos_lib::sync::spin::barrier::Barrier;

const MPSTART_RELOC_ADDRESS: *mut u8 = 0x8000 as _;

extern "C" {
    static MPSTART_16: u8;
    static MPSTART_16_LEN: usize;
}

type MpStartFn = fn(u8, *const ()) -> !;

static mut MPSTART_FN: MaybeUninit<MpStartFn> = MaybeUninit::uninit();
static mut MPSTART_USER: *const () = null();
static mut MPSTART_APIC_BASE: usize = 0;
static mut MPSTART_BARRIER: MaybeUninit<Barrier> = MaybeUninit::uninit();

#[no_mangle]
static mut MPSTART_PDT4: usize = 0;
#[no_mangle]
static MPSTART_STACK_BASE: usize = 0x9000;
#[no_mangle]
static MPSTART_STACK_STRIDE: usize = 0x4000;

unsafe fn start_processor(apic: &mut Apic, lapic_id: u8) {
    let mpstart_page = MPSTART_RELOC_ADDRESS as usize >> PAGE_SHIFT;
    apic.start_ap(lapic_id, mpstart_page, |d| {
        super::timer::delay(d).unwrap()
    });
}

pub unsafe fn start_mp(madt: &madt::Madt, start_fn: MpStartFn, user: *const ()) -> usize {
    assert!(
        MPSTART_16_LEN <= 0x78000,
        "MPSTART is too big, must fit in [0x8000, 0x80000)"
    );
    core::ptr::copy_nonoverlapping(
        &MPSTART_16,
        MPSTART_RELOC_ADDRESS as *mut u8,
        MPSTART_16_LEN,
    );

    let apic = super::intr::apic();

    let this_apic_id = apic.id();

    let entries = madt.entries().filter_map(|e| {
        if let madt::Entry::LApic(lapic) = e {
            Some(lapic.apic_id)
        } else {
            None
        }
    });
    let count = madt.apic_count();

    MPSTART_FN = MaybeUninit::new(start_fn);
    MPSTART_USER = user;
    MPSTART_APIC_BASE = madt.lapic_address as usize;
    MPSTART_BARRIER = MaybeUninit::new(Barrier::new(count));

    MPSTART_PDT4 = Cr3::read()
        .0
        .addr()
        .as_u64() as usize;

    for lapic_id in entries {
        if lapic_id != this_apic_id as u8 {
            start_processor(apic, lapic_id);
        }
    }

    MPSTART_BARRIER.assume_init_ref().wait();

    count
}

#[no_mangle]
extern "C" fn secondary_main() -> ! {
    unsafe {
        let apic = Apic::with_address(MPSTART_APIC_BASE);
        let id = apic.id();
        MPSTART_BARRIER.assume_init_ref().wait();
        (MPSTART_FN.assume_init())(id, MPSTART_USER);
    }
}
