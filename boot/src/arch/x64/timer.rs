use core::convert::TryInto;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::Relaxed;
use core::time::Duration;

use chos_lib::sync::spin::sem::SpinSem;
use x86_64::structures::idt::InterruptStackFrame;

use super::acpi::hpet::HPET;

static DONE: SpinSem = SpinSem::new(0);

const IOAPIC_TIMER_ROUTE: u8 = 8;

extern "x86-interrupt" fn timer_callback(_: InterruptStackFrame) {
    DONE.signal();
    unsafe {
        super::intr::eoi();
    }
}

static mut HPET: Option<chos_lib::arch::x64::hpet::HPET> = None;

pub fn initialize(hpet_table: &HPET) {
    super::intr::try_ioapic_alloc(IOAPIC_TIMER_ROUTE, |_| (), timer_callback)
        .expect("Could not allocate IOApic interrupt 8");
    unsafe {
        HPET = Some(chos_lib::arch::x64::hpet::HPET::with_address(hpet_table.address));
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DelayInProgressError;

static IN_PROGRESS: AtomicBool = AtomicBool::new(false);

pub fn delay(d: Duration) -> Result<(), DelayInProgressError> {
    if let Err(_) = IN_PROGRESS.compare_exchange(false, true, Relaxed, Relaxed) {
        return Err(DelayInProgressError);
    }
    let hpet = unsafe { &mut HPET }
        .as_mut()
        .expect("Timer not initialized");
    let period = hpet.period() as u128;
    let mut tim0 = hpet.get_timer(0);

    let comparator = (d.as_nanos() * 1_000_000 + period - 1) / period;
    let comparator: u64 = comparator.try_into().unwrap();

    unsafe {
        tim0.set_comparator(comparator);
        tim0.set_int_route(IOAPIC_TIMER_ROUTE);
        tim0.enable();

        drop(tim0);

        hpet.set_count(0);

        hpet.enable();
    }

    DONE.wait();

    unsafe {
        let mut tim0 = hpet.get_timer(0);
        tim0.disable();
        hpet.disable();
    };

    IN_PROGRESS.store(false, Relaxed);
    Ok(())
}
