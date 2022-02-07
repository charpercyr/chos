use alloc::vec::Vec;

use chos_lib::arch::mm::VAddr;
use chos_lib::elf::{Elf, SymtabEntryType};
use chos_lib::init::ConstInit;
use chos_lib::log::debug;
use chos_lib::pool::PoolBox;
use chos_lib::sync::SpinRWLock;
use intrusive_collections::rbtree::{self, RBTree};
use intrusive_collections::{Bound, KeyAdapter};

use crate::mm::slab::DefaultPoolObjectAllocator;

struct ElfSymbols {
    link: rbtree::AtomicLink,
    base: VAddr,
    symbols: Vec<(u64, usize)>,
    elf_data: Vec<u8>,
}

impl PartialEq for ElfSymbols {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}
impl Eq for ElfSymbols {}

impl PartialOrd for ElfSymbols {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ElfSymbols {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.base.cmp(&other.base)
    }
}

static ELF_SYMBOLS_POOL: DefaultPoolObjectAllocator<ElfSymbols, 0> =
    DefaultPoolObjectAllocator::INIT;
chos_lib::pool!(struct ElfSymbolsPool: ElfSymbols => &ELF_SYMBOLS_POOL);

type ElfSymbolBox = PoolBox<ElfSymbols, ElfSymbolsPool>;

chos_lib::intrusive_adapter!(ElfSymbolsAdapter = ElfSymbolBox: ElfSymbols { link: rbtree::AtomicLink });
impl<'a> KeyAdapter<'a> for ElfSymbolsAdapter {
    type Key = VAddr;
    fn get_key(
        &self,
        value: &'a <Self::PointerOps as intrusive_collections::PointerOps>::Value,
    ) -> Self::Key {
        value.base
    }
}

static SYMBOLS: SpinRWLock<RBTree<ElfSymbolsAdapter>> =
    SpinRWLock::new(RBTree::new(ElfSymbolsAdapter::new()));

pub fn add_elf_symbols(base: VAddr, elf: &Elf) {
    if let Some(symtab) = elf
        .sections()
        .iter()
        .find(|s| s.name(&elf) == Some(".symtab"))
        .and_then(|s| s.as_symtab(&elf))
    {
        let sym_iter = symtab
            .iter()
            .enumerate()
            .filter(|(_, s)| s.typ() == SymtabEntryType::Func);
        let sym_count = sym_iter.clone().count();
        let mut symbols = Vec::with_capacity(sym_count);

        debug!("Adding {} symbols to lookup", sym_count);

        let mut needs_sorting = false;
        let mut last_value: u64 = 0;
        symbols.extend(sym_iter.map(|(i, sym)| {
            if sym.value() < last_value {
                needs_sorting = true;
            }
            last_value = sym.value();
            (sym.value(), i)
        }));
        debug_assert_eq!(symbols.len(), sym_count);

        if needs_sorting {
            symbols.sort_unstable_by_key(|(k, _)| *k);
        }

        SYMBOLS.lock_write().insert(ElfSymbolBox::new(ElfSymbols {
            base,
            link: rbtree::AtomicLink::new(),
            symbols,
            elf_data: elf.data().into(),
        }));
    }
}

fn lookup_symbol_impl<R>(
    elf_symbols: &RBTree<ElfSymbolsAdapter>,
    address: VAddr,
    callback: impl FnOnce(&str, VAddr, u64) -> R,
) -> Option<R> {
    elf_symbols
        .upper_bound(Bound::Included(&address))
        .get()
        .and_then(|syms| {
            let value = (address - syms.base).as_u64();
            let sym_array_idx = match syms.symbols.binary_search_by_key(&value, |(k, _)| *k) {
                Ok(sym_idx) => sym_idx,
                Err(0) => return None,
                Err(sym_idx) => sym_idx - 1,
            };
            let sym_idx = syms.symbols[sym_array_idx].1;
            let elf = unsafe { Elf::new_unchecked(&syms.elf_data) };
            elf.sections()
                .symtab_strtab(&elf)
                .and_then(|(symtab, strtab)| {
                    let sym = symtab.get(sym_idx);
                    sym.name(&strtab).map(|name| {
                        callback(
                            name,
                            syms.base + sym.value(),
                            (address - (syms.base + sym.value())).as_u64(),
                        )
                    })
                })
        })
}

pub fn lookup_symbol<R>(address: VAddr, callback: impl FnOnce(&str, VAddr, u64) -> R) -> Option<R> {
    let elf_symbols = SYMBOLS.lock_read();
    lookup_symbol_impl(&elf_symbols, address, callback)
}

pub unsafe fn lookup_symbol_unlocked<R>(
    address: VAddr,
    callback: impl FnOnce(&str, VAddr, u64) -> R,
) -> Option<R> {
    lookup_symbol_impl(&*SYMBOLS.get_ptr(), address, callback)
}
