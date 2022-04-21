use core::mem::{align_of, size_of};
use core::slice::from_raw_parts;

use chos_lib::elf::Elf;
use chos_lib::mm::VAddr;
use chos_lib::ptr::dangling;

#[derive(Copy, Clone)]
pub struct Module {
    pub decl: &'static ModuleDecl,
}

pub struct ModuleDecl {
    name: &'static str,
    init: Option<fn(Module)>,
    fini: Option<fn()>,
}

impl ModuleDecl {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            init: None,
            fini: None,
        }
    }

    pub const fn with_init_fini(self, init: fn(Module), fini: fn()) -> Self {
        Self {
            name: self.name,
            init: Some(init),
            fini: Some(fini),
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub fn init(&self) -> Option<fn(Module)> {
        self.init
    }

    pub fn fini(&self) -> Option<fn()> {
        self.fini
    }
}

pub macro __module_section() {
    ".chos.module"
}

pub macro module_decl($m:expr) {
    #[used]
    #[link_section = $crate::module::__module_section!()]
    static __CHOS_MODULE: $crate::module::ModuleDecl = $m as ModuleDecl;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InvalidModuleSection {
    BadAlignment,
    BadSize,
}

pub fn get_modules_for_elf(
    elf: &Elf,
    base: VAddr,
) -> Result<&'static [ModuleDecl], InvalidModuleSection> {
    for sec in elf.sections() {
        if sec.name(elf) == Some(__module_section!()) {
            if (sec.addr_align() as usize) < align_of::<ModuleDecl>() {
                return Err(InvalidModuleSection::BadAlignment);
            }
            if (sec.size() as usize) % size_of::<ModuleDecl>() != 0 {
                return Err(InvalidModuleSection::BadSize);
            }
            let base = base + sec.addr();
            if base.as_usize() % align_of::<ModuleDecl>() != 0 {
                return Err(InvalidModuleSection::BadAlignment);
            }
            return Ok(unsafe {
                base.from_raw_parts((sec.size() as usize) / size_of::<ModuleDecl>())
            });
        }
    }
    Ok(unsafe { from_raw_parts(dangling(), 0) })
}
