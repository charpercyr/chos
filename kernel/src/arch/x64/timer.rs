use core::mem::MaybeUninit;

use chos_config::arch::mm::virt;
use chos_config::timer::TICKS_HZ;
use chos_lib::arch::acpi::Rsdt;
use chos_lib::arch::hpet::{Hpet, TimerType};
use chos_lib::arch::ioapic;
use chos_lib::arch::tables::StackFrame;
use chos_lib::int::CeilDiv;
use chos_lib::log::debug;

use super::intr::allocate_ioapic_interrupt;
use crate::kmain::KernelArgs;
use crate::mm::this_cpu_info;
use crate::timer::{on_tick, on_tick_main_cpu, NS_PER_TICKS};

const PIT_TIMER_IOAPIC_INTR: u8 = 2;
const APIC_INTR_MASK: u32 = !(1 << PIT_TIMER_IOAPIC_INTR);

static mut HPET: MaybeUninit<Hpet> = MaybeUninit::uninit();

fn timer_intr_handler(_: StackFrame) {
    let id = this_cpu_info().id;
    if id == 0 {
        on_tick_main_cpu();
    } else {
        on_tick()
    }
}

pub fn arch_init_timer(args: &KernelArgs) {
    let rsdt = unsafe { Rsdt::new_offset(args.arch.rsdt, virt::PHYSICAL_MAP_BASE.addr()) };
    let hpet_tbl = rsdt.hpet().expect("Need HPET table");

    let mut hpet = unsafe { Hpet::new(virt::DEVICE_BASE.addr() + hpet_tbl.address as u64) };

    unsafe {
        let period = hpet.period();
        let comparator = (NS_PER_TICKS * 1_000_000).ceil_div(period as u64);

        debug!(
            "HPET Period is {}fs, setting comparator to {} ({}Hz)",
            period, comparator, TICKS_HZ
        );

        hpet.disable();

        let mut timer = hpet.get_timer_mut(0);

        let mask = timer.int_route_mask() & APIC_INTR_MASK;
        let intr = allocate_ioapic_interrupt(
            mask as u64,
            timer_intr_handler,
            ioapic::Destination::Logical((1 << args.core_count) - 1),
        )
        .expect("No interrupt free for timer");

        timer.set_int_route(intr);
        timer.set_type(TimerType::Periodic);
        timer.enable();
        timer.set_comparator(comparator);

        hpet.set_count(0);
        hpet.enable();
    }

    unsafe { HPET = MaybeUninit::new(hpet) };
}
