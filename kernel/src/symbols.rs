use alloc::string::String;

use chos_lib::arch::mm::VAddr;
use chos_lib::elf::{Elf, SymtabEntryType};
use chos_lib::init::ConstInit;
use chos_lib::pool::PoolBox;
use chos_lib::sync::SpinRWLock;
use intrusive_collections::rbtree::{self, RBTree};
use intrusive_collections::{Bound, KeyAdapter};

use crate::mm::slab::DefaultPoolObjectAllocator;

struct Symbol {
    address: VAddr,
    link: rbtree::AtomicLink,
    name: String,
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
impl Eq for Symbol {}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.address.cmp(&other.address)
    }
}

static SYMBOL_POOL: DefaultPoolObjectAllocator<Symbol, 0> = DefaultPoolObjectAllocator::INIT;
chos_lib::pool!(struct SymbolPool: Symbol => &SYMBOL_POOL);

type SymbolBox = PoolBox<Symbol, SymbolPool>;

chos_lib::intrusive_adapter!(SymbolAdapter = SymbolBox: Symbol { link: rbtree::AtomicLink });
impl<'a> KeyAdapter<'a> for SymbolAdapter {
    type Key = VAddr;
    fn get_key(
        &self,
        value: &'a <Self::PointerOps as intrusive_collections::PointerOps>::Value,
    ) -> Self::Key {
        value.address
    }
}

static SYMBOLS: SpinRWLock<RBTree<SymbolAdapter>> =
    SpinRWLock::new(RBTree::new(SymbolAdapter::new()));

fn add_symbol_to_tree(symbols: &mut RBTree<SymbolAdapter>, address: VAddr, name: String) {
    symbols.insert(PoolBox::new(Symbol {
        address,
        name: String::new(),
        link: rbtree::AtomicLink::new(),
    }));
}

pub fn add_elf_symbols_to_tree(base: VAddr, elf: &Elf) {
    let symtab = elf
        .sections()
        .iter()
        .find(|s| s.name(elf) == Some(".symtab"))
        .and_then(|s| s.as_symtab(elf));
    let strtab = elf
        .sections()
        .iter()
        .find(|s| s.name(elf) == Some(".strtab"))
        .and_then(|s| s.as_strtab(elf));
    if let Some((symtab, strtab)) = symtab.zip(strtab) {
        let mut symbols = SYMBOLS.lock_write();
        for sym in symtab {
            if sym.typ() == SymtabEntryType::Func {
                if let Some(name) = sym.name(&strtab) {
                    add_symbol_to_tree(&mut symbols, base + sym.value(), name.into())
                }
            }
        }
    }
}

fn lookup_symbol_in_tree<R>(
    symbols: &RBTree<SymbolAdapter>,
    address: VAddr,
    callback: impl FnOnce(&str, VAddr, u64) -> R,
) -> Option<R> {
    let sym = symbols.upper_bound(Bound::Included(&address));
    sym.get()
        .map(|sym| callback(&sym.name, sym.address, (address - sym.address).as_u64()))
}

pub fn lookup_symbol<R>(address: VAddr, callback: impl FnOnce(&str, VAddr, u64) -> R) -> Option<R> {
    let symbols = SYMBOLS.lock_read();
    lookup_symbol_in_tree(&symbols, address, callback)
}

pub unsafe fn lookup_symbol_unlocked<R>(
    address: VAddr,
    callback: impl FnOnce(&str, VAddr, u64) -> R,
) -> Option<R> {
    lookup_symbol_in_tree(&*SYMBOLS.get_ptr(), address, callback)
}
