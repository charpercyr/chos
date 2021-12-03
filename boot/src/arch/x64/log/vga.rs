use core::fmt;
use core::mem::transmute;

use chos_lib::{copy_volatile, Volatile};

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum VgaColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    LightBrown = 14,
    White = 15,
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct VgaEntry(u16);

impl VgaEntry {
    pub const EMPTY: Self = Self::new(b' ', VgaColor::White, VgaColor::Black);
    pub const fn new(c: u8, fg: VgaColor, bg: VgaColor) -> Self {
        let fg: u8 = unsafe { transmute(fg) };
        let bg: u8 = unsafe { transmute(bg) };
        Self((c as u16) | ((fg as u16) << 8) | ((bg as u16) << 12))
    }

    pub const fn character(&self) -> u8 {
        (self.0 & 0xff) as u8
    }

    pub const fn fg(&self) -> VgaColor {
        unsafe { transmute((self.0 & 0xf00 >> 8) as u8) }
    }

    pub const fn bg(&self) -> VgaColor {
        unsafe { transmute((self.0 & 0xf000 >> 12) as u8) }
    }
}

impl fmt::Debug for VgaEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VgaEntry")
            .field("character", &self.character())
            .field("fg", &self.fg())
            .field("bg", &self.bg())
            .finish()
    }
}

#[derive(Debug)]
pub struct Vga {
    x: usize,
    y: usize,
}
pub static mut VGA: Vga = Vga::new();

impl Vga {
    const BASE: *mut Volatile<VgaEntry> = 0xb8000 as _;
    const WIDTH: usize = 80;
    const HEIGHT: usize = 25;

    const fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn put(&mut self, entry: VgaEntry) {
        unsafe {
            (*Self::BASE.add(self.x + self.y * Self::WIDTH)).write(entry);
        }
        self.x += 1;
        if self.x >= Self::WIDTH {
            self.newline();
        }
    }

    pub fn newline(&mut self) {
        self.x = 0;
        self.y += 1;
        if self.y >= Self::HEIGHT {
            self.scroll();
        }
    }

    pub fn scroll(&mut self) {
        self.scroll_n(1);
    }

    pub fn scroll_n(&mut self, mut n: usize) {
        assert!(n <= Self::HEIGHT);
        if n > self.y {
            n = self.y;
        }
        self.x = 0;
        self.y -= n;
        unsafe {
            copy_volatile(
                Self::BASE.add(n * Self::WIDTH),
                Self::BASE,
                (Self::HEIGHT - n) * Self::WIDTH,
            );
        }
    }

    pub fn clear(&mut self) {
        self.x = 0;
        self.y = 0;
        for i in 0..(Self::WIDTH * Self::HEIGHT) {
            unsafe { (*Self::BASE.add(i)).write(VgaEntry::EMPTY) }
        }
    }
}

impl fmt::Write for Vga {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            match b {
                b'\n' => self.newline(),
                b'\t' => {
                    for _ in 0..(4 - self.x % 4) {
                        self.put(VgaEntry::new(b' ', VgaColor::White, VgaColor::Black));
                    }
                }
                0x20..=0x7e => self.put(VgaEntry::new(b, VgaColor::White, VgaColor::Black)),
                _ => self.put(VgaEntry::new(0xfe, VgaColor::White, VgaColor::Black)),
            }
        }
        Ok(())
    }
}

impl super::Output for Vga {
    fn init(&mut self) {
        self.clear();
    }
}
