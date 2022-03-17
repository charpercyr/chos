use core::intrinsics::size_of;
use core::mem::align_of;
use core::slice;

use chos_lib::log::todo_warn;
use chos_lib::{arch::mm::VAddr, log::warn};
use chos_lib::elf::Elf;
use chos_lib::sync::Spinlock;
use intrusive_collections::{intrusive_adapter, linked_list};

macro init_fn_section() {
    ".init_fn"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EarlyPhase {
    EarlyMem,
    PreUser,
}
// Keep in sync with EarlyPhase
const N_EARLY_PHASE: usize = 1;

const INIT_FNS_LIST: Spinlock<linked_list::LinkedList<InitFnAdapter>> =
    Spinlock::new(linked_list::LinkedList::new(InitFnAdapter::NEW));
static EARLY_INIT_FNS: [Spinlock<linked_list::LinkedList<InitFnAdapter>>; N_EARLY_PHASE] =
    [INIT_FNS_LIST; N_EARLY_PHASE];

impl EarlyPhase {
    fn index(self) -> usize {
        match self {
            Self::EarlyMem => 0,
            Self::PreUser => 1,
        }
    }
}

pub struct InitFn {
    pub link: linked_list::AtomicLink,
    pub init: fn(),
}

pub struct InitFnStatic {
    pub init_fn: InitFn,
    pub phase: EarlyPhase,
}

intrusive_adapter!(InitFnAdapter = &'static InitFn : InitFn { link: linked_list::AtomicLink });

pub fn add_init_fn(phase: EarlyPhase, init_fn: &'static InitFn) {
    todo!()
}

pub unsafe fn add_elf_init_fns(elf: &Elf, base: VAddr) {
    for sec in elf.sections() {
        if sec.name(&elf) == Some(init_fn_section!()) {
            let addr = sec.addr() + base;
            let len = sec.size() as usize;
            assert_eq!(
                addr.as_usize() % align_of::<InitFnStatic>(),
                0,
                concat!(init_fn_section!(), " section should be properly aligned")
            );
            assert_eq!(
                len % size_of::<InitFnStatic>(),
                0,
                concat!(init_fn_section!(), " section should be properly sized")
            );
            let init_fns: &'static [InitFnStatic] = addr.from_raw_parts(len);
            init_fns.iter().for_each(|init_fn| add_init_fn(init_fn.phase, &init_fn.init_fn));
            return;
        }
    }
    todo_warn!("No init fns")
}

pub macro init_fn($phase:expr => $name:path) {
    paste::item! {
        #[link_section = init_fn_section!()]
        #[used]
        static [<__INIT_FN_ $name:snake:upper>]: $crate::sched::early::InitFnStatic = $crate::sched::early::InitFnStatic {
            init_fn: $crate::sched::early::InitFn {
                link: intrusive_collections::linked_list::AtomicLink::new(),
                init: $name,
            },
            phase: $phase,
        };
    }
}

