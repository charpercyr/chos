use crate::{raw::Elf64GnuHash, StrTab, Symtab, SymtabEntry};

use core::mem::size_of;
use core::slice::from_raw_parts;

#[derive(Clone, Copy)]
pub struct GnuHash<'a> {
    hdr: &'a Elf64GnuHash,
    bloom: &'a [u64],
    buckets: &'a [u32],
    chain: &'a [u32],
}
impl<'a> GnuHash<'a> {
    pub unsafe fn new(buf: &'a [u8]) -> Self {
        let mut ptr = buf.as_ptr();
        let hdr: &Elf64GnuHash = &*buf.as_ptr().cast();
        ptr = ptr.add(size_of::<Elf64GnuHash>());
        let bloom = from_raw_parts(ptr.cast(), hdr.bloom_size as usize);
        ptr = ptr.add(hdr.bloom_size as usize * size_of::<u64>());
        let buckets = from_raw_parts(ptr.cast(), hdr.nbuckets as usize);
        ptr = ptr.add(hdr.nbuckets as usize * size_of::<u32>());
        let chain_size = buf.as_ptr().add(buf.len()).offset_from(ptr) as usize;
        let chain = from_raw_parts(ptr.cast(), chain_size);
        Self {
            hdr,
            bloom,
            buckets,
            chain,
        }
    }

    // From https://flapenguin.me/elf-dt-gnu-hash
    pub fn lookup(
        &self,
        name: &str,
        strtab: &'a StrTab<'a>,
        symtab: &'a Symtab<'a>,
    ) -> Option<SymtabEntry<'a>> {
        let namehash = gnu_hash(name);

        let word = self.bloom[(namehash as usize / 64) % self.hdr.bloom_size as usize];
        let mask = (1u64.wrapping_shl(namehash % self.hdr.bloom_size))
            | (1u64
                .wrapping_shl((namehash.wrapping_shr(self.hdr.bloom_shift)) % self.hdr.bloom_size));

        if (word & mask) != mask {
            return None;
        }

        let mut symidx = self.buckets[(namehash % self.hdr.nbuckets) as usize];
        if symidx < self.hdr.symoffset {
            return None;
        }

        loop {
            let hash = self.chain[(symidx - self.hdr.symoffset) as usize];
            let sym_entry = symtab.get(symidx as usize);
            if let Some(symname) = sym_entry.name(strtab) {
                if (namehash | 1) == (hash | 1) && name == symname {
                    return Some(sym_entry);
                }
            }
            if (hash & 1) != 0 {
                return None;
            }

            symidx += 1;
        }
    }
}

fn gnu_hash(n: impl AsRef<[u8]>) -> u32 {
    let mut h: u32 = 5381;
    for &b in n.as_ref() {
        h = h.wrapping_shl(5).wrapping_add(h).wrapping_add(b as u32);
    }
    h
}
