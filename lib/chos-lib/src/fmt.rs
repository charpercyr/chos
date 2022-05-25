use core::fmt::{self, Write};
use core::mem::size_of;
use core::slice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub u64);

impl Bytes {
    fn split(self) -> (u16, u16, u16, u16) {
        let mut b = self.0;
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        let gb = b / GB;
        b -= gb * GB;
        let mb = b / MB;
        b -= mb * MB;
        let kb = b / KB;
        b -= kb * KB;
        (gb as u16, mb as u16, kb as u16, b as u16)
    }
}

fn fmt_bytes(
    b: Bytes,
    f: &mut fmt::Formatter,
    fmt: impl Fn(&u16, &mut fmt::Formatter) -> fmt::Result,
) -> fmt::Result {
    let (gb, mb, kb, b) = b.split();
    f.write_str("(")?;
    if gb != 0 {
        fmt(&gb, f)?;
        f.write_str("GiB ")?;
    }
    if gb != 0 || mb != 0 {
        fmt(&mb, f)?;
        f.write_str("MiB ")?;
    }
    if gb != 0 || mb != 0 || kb != 0 {
        fmt(&kb, f)?;
        f.write_str("KiB ")?;
    }
    fmt(&b, f)?;
    f.write_str("B)")
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bytes(*self, f, fmt::Display::fmt)
    }
}

impl fmt::LowerHex for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bytes(*self, f, fmt::LowerHex::fmt)
    }
}

impl fmt::UpperHex for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bytes(*self, f, fmt::UpperHex::fmt)
    }
}

impl fmt::Octal for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bytes(*self, f, fmt::Octal::fmt)
    }
}

impl fmt::Binary for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_bytes(*self, f, fmt::Binary::fmt)
    }
}

#[derive(Debug)]
pub struct SizeWriter {
    size: usize,
}

impl SizeWriter {
    pub const fn new() -> Self {
        Self { size: 0 }
    }

    pub const fn size(&self) -> usize {
        self.size
    }
}

impl fmt::Write for SizeWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.size += s.len();
        Ok(())
    }
}

pub fn size_of_fmt(fmt: fmt::Arguments) -> usize {
    let mut size_writer = SizeWriter::new();
    size_writer
        .write_fmt(fmt)
        .expect("SizeWriter cannot (should not) fail");
    size_writer.size()
}

#[cfg(test)]
mod tests {
    use core::fmt::Arguments;

    use super::*;

    #[test]
    fn size_writer() {
        fn test_one(args: Arguments, expected: usize) {
            let size = size_of_fmt(args);
            assert_eq!(size, expected, "Failed for '{}'", args);
        }
        test_one(format_args!(""), 0);
        test_one(format_args!("hello"), 5);
        test_one(format_args!("{}", 1), 1);
        test_one(format_args!("{:b}+{:#x}", 10, 16), 9);
        test_one(format_args!("hello {}", "world"), 11);
    }
}

fn hexdump(bytes: &[u8], f: &mut fmt::Formatter) -> fmt::Result {
    f.write_fmt(format_args!("[{:p}]\n", bytes.as_ptr()))?;
    for (i, block) in bytes.chunks(16).enumerate() {
        f.write_fmt(format_args!("[0x{:04x}]", i * 16))?;
        for &b in block {
            f.write_fmt(format_args!(" {:02x}", b))?;
        }
        for _ in 0..(16 - block.len()) {
            f.write_str("   ")?;
        }
        f.write_str("  [")?;
        for &b in block {
            if b.is_ascii() && !b.is_ascii_control() {
                f.write_char(b as char)?;
            } else {
                f.write_char('.')?;
            }
        }
        for _ in 0..(16 - block.len()) {
            f.write_char(' ')?;
        }
        f.write_str("]\n")?;
    }
    Ok(())
}

pub struct Hexdump<'a, T>(pub &'a [T]);

impl<T> fmt::Display for Hexdump<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            hexdump(
                slice::from_raw_parts(self.0.as_ptr().cast(), self.0.len() * size_of::<T>()),
                f,
            )
        }
    }
}
