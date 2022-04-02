use core::convert::TryInto;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;

use chos_lib::arch::tables::{interrupt, StackFrame};
use chos_lib::arch::x64::acpi::hpet::Hpet;
use chos_lib::int::CeilDiv;
use chos_lib::mm::VAddr;
use chos_lib::sync::{Sem, SpinSem};

static DONE: SpinSem = SpinSem::zero();

const IOAPIC_TIMER_ROUTE: u8 = 8;

#[interrupt]
extern "x86-interrupt" fn timer_callback(_: StackFrame) {
    DONE.signal();
    unsafe {
        super::intr::eoi();
    }
}

static mut HPET: Option<chos_lib::arch::x64::hpet::Hpet> = None;

pub fn initialize(hpet_table: &Hpet) {
    super::intr::try_ioapic_alloc(IOAPIC_TIMER_ROUTE, |_| (), timer_callback)
        .expect("Could not allocate IOApic interrupt 8");
    unsafe {
        HPET = Some(chos_lib::arch::x64::hpet::Hpet::new(VAddr::from_usize(
            hpet_table.address,
        )));
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DelayInProgressError;

pub fn delay(d: Duration) -> Result<(), DelayInProgressError> {
    static IN_PROGRESS: AtomicBool = AtomicBool::new(false);
    IN_PROGRESS
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .map_err(|_| DelayInProgressError)?;
    let hpet = unsafe { HPET.as_mut() }.expect("Timer not initialized");
    let period = hpet.period() as u128;
    let mut tim0 = hpet.get_timer_mut(0);

    let comparator = (d.as_nanos() * 1_000_000).ceil_div(period);
    let comparator: u64 = comparator.try_into().unwrap();

    unsafe {
        tim0.set_comparator(comparator);
        tim0.set_int_route(IOAPIC_TIMER_ROUTE);
        tim0.enable();

        hpet.set_count(0);

        hpet.enable();
    }

    DONE.wait();

    unsafe {
        let mut tim0 = hpet.get_timer_mut(0);
        tim0.disable();
        hpet.disable();
    };

    IN_PROGRESS.store(false, Ordering::Relaxed);
    Ok(())
}
