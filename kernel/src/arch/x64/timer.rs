use core::mem::MaybeUninit;

use chos_config::arch::mm::virt;
use chos_lib::arch::acpi::Rsdt;
use chos_lib::arch::hpet::{Hpet, TimerType};
use chos_lib::arch::ioapic;
use chos_lib::arch::tables::InterruptStackFrame;
use chos_lib::int::CeilDiv;

use super::intr::allocate_ioapic_interrupt;
use crate::kmain::KernelArgs;
use crate::mm::this_cpu_info;
use crate::timer::{on_tick, on_tick_main_cpu, NS_PER_TICKS};

static mut HPET: MaybeUninit<Hpet> = MaybeUninit::uninit();

fn timer_callback(_: InterruptStackFrame) {
    let id = this_cpu_info().id;
    if id == 0 {
        on_tick_main_cpu();
    } else {
        on_tick()
    }
}

pub fn arch_init_timer(args: &KernelArgs) {
    let rsdt = unsafe { Rsdt::new_offset(args.arch.rsdt, virt::PHYSICAL_MAP_BASE) };
    let hpet_tbl = rsdt.hpet().expect("Need HPET table");

    let mut hpet = unsafe { Hpet::new(virt::DEVICE_BASE + hpet_tbl.address as u64) };

    unsafe {
        let comparator = (NS_PER_TICKS * 1_000_000).ceil_div(hpet.period() as u64);

        let mut tim0 = hpet.get_timer(0);

        let mask = tim0.int_route_mask();
        let intr = allocate_ioapic_interrupt(
            mask as u64,
            timer_callback,
            ioapic::Destination::Logical((1 << args.core_count) - 1),
        )
        .expect("No interrupt free for timer");

        tim0.set_int_route(intr);
        tim0.set_type(TimerType::Periodic);
        tim0.set_comparator(comparator);
        tim0.enable();

        drop(tim0);

        hpet.set_count(0);
        hpet.enable();
    }

    unsafe { HPET = MaybeUninit::new(hpet) };
}
