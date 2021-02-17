use core::mem::size_of;

#[repr(C, packed)]
pub struct MultibootHeader {
    pub total_size: u32,
    pub _reserved: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct RSDP1 {
    pub sig: [u8; 8],
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub rev: u8,
    pub rsdt: u32,
}

#[derive(Debug, Copy, Clone)]
pub enum MultibootTag {
    BasicMemory {
        mem_lower: u32,
        mem_upper: u32,
    },
    BIOSBootDevice {
        biosdev: u32,
        partition: u32,
        sub_partition: u32,
    },
    BootCommandLine {
        cmdline: *const u8,
    },
    Module {
        mod_start: u32,
        mod_end: u32,
        name: *const u8,
    },
    MemoryMap(MemoryMapParser),
    BootLoaderName {
        name: *const u8,
    },
    RSDP1(RSDP1),
    ImageLoad {
        base: u32,
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MultibootParser {
    cur: *const u8,
    end: *const u8,
}

impl Iterator for MultibootParser {
    type Item = MultibootTag;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            unsafe {
                if self.cur >= self.end {
                    return None;
                }
                let entry: &TagHeader = &*self.cur.cast();
                let data = self.cur.offset(size_of::<TagHeader>() as isize);
                let tag = match entry.ty {
                    1 => Some(MultibootTag::BootCommandLine {
                        cmdline: data.cast(),
                    }),
                    2 => Some(MultibootTag::BootLoaderName { name: data.cast() }),
                    3 => {
                        let mod_start: u32 = *data.cast();
                        let mod_end: u32 = *data.offset(4).cast();
                        let name: *const u8 = data.offset(8);
                        Some(MultibootTag::Module {
                            mod_start,
                            mod_end,
                            name,
                        })
                    }
                    4 => {
                        let mem_lower: u32 = *data.cast();
                        let mem_upper: u32 = *data.offset(4).cast();
                        Some(MultibootTag::BasicMemory {
                            mem_lower: mem_lower * 1024,
                            mem_upper: mem_upper * 1024,
                        })
                    }
                    5 => {
                        let biosdev: u32 = *data.cast();
                        let partition: u32 = *data.offset(4).cast();
                        let sub_partition: u32 = *data.offset(8).cast();
                        Some(MultibootTag::BIOSBootDevice {
                            biosdev,
                            partition,
                            sub_partition,
                        })
                    }
                    6 => {
                        let entry_size: u32 = *data.cast();
                        let end = self.cur.offset(entry.size as isize);
                        Some(MultibootTag::MemoryMap(MemoryMapParser {
                            cur: data.offset(8),
                            end,
                            entry_size: entry_size as usize,
                        }))
                    },
                    14 => {
                        Some(MultibootTag::RSDP1(core::ptr::read(data.cast())))
                    },
                    21 => {
                        Some(MultibootTag::ImageLoad {
                            base: *data.cast(),
                        })
                    },
                    _ => None,
                };
                self.cur = self.cur.offset(entry.size as isize);
                self.cur = self.cur.offset(self.cur.align_offset(8) as isize);
                if tag.is_some() {
                    return tag;
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MemoryMapParser {
    entry_size: usize,
    cur: *const u8,
    end: *const u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MemoryEntryType {
    Available = 1,
    Reserved = 2,
    ACPI = 3,
    ReserveHibernate = 4,
    Defective = 5,
}

pub struct MemoryEntry {
    pub ty: MemoryEntryType,
    pub start: usize,
    pub end: usize,
}

impl Iterator for MemoryMapParser {
    type Item = MemoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.end {
            None
        } else {
            unsafe {
                let base: u64 = *self.cur.cast();
                let length: u64 = *self.cur.offset(8).cast();
                let ty: u32 = *self.cur.offset(16).cast();
                let ty = match ty {
                    1 => MemoryEntryType::Available,
                    3 => MemoryEntryType::ACPI,
                    4 => MemoryEntryType::ReserveHibernate,
                    5 => MemoryEntryType::Defective,
                    _ => MemoryEntryType::Reserved,
                };
                self.cur = self.cur.offset(self.entry_size as isize);
                Some(MemoryEntry {
                    ty,
                    start: base as usize,
                    end: (base + length) as usize,
                })
            }
        }
    }
}

pub unsafe fn parse_mb(header: *const MultibootHeader) -> MultibootParser {
    let size = (*header).total_size;
    let cur: *const u8 = header.cast();
    MultibootParser {
        cur: cur.offset(size_of::<MultibootHeader>() as isize),
        end: cur.offset(size as isize),
    }
}

#[repr(C, packed)]
struct TagHeader {
    ty: u32,
    size: u32,
}
