use std::{cmp::Ordering, hint::unreachable_unchecked};

/// Expands 8-bit integer to 16-bit like this:
/// 0b01010111 -> 0b0011001100111111
/// 0b10101010 -> 0b1100110011001100
fn expand(n: u8) -> u16 {
    let mut result: u16 = 0;
    for i in 0..8 {
        result |= (n as u16 & (1 << i)) << i
    }
    result | (result << 1)
}

pub struct Display {
    d: Box<[u128; 64]>,
    hi_res: bool,
    dirty: bool,
}

impl Display {
    pub fn new() -> Display {
        Display {
            d: Box::new([0; 64]),
            hi_res: false,
            dirty: false,
        }
    }

    pub fn hi_res_mode(&mut self) {
        self.hi_res = true
    }

    pub fn low_res_mode(&mut self) {
        self.hi_res = false
    }

    pub fn scroll_down(&mut self, n: u32) {
        self.d.rotate_right(n as usize); // TODO: fix this (probably not the best solution)
        self.d[0] = 0;
        self.d[1] = 0
    }

    /// DO NOT USE WITH n = 0, IT'S UNDEFINED BEHAVIOR
    pub fn scroll_side(&mut self, n: i32) {
        for row in &mut *self.d {
            match n.cmp(&0) {
                Ordering::Greater => *row = row.rotate_right(n as u32),
                Ordering::Less => *row = row.rotate_left(n.abs() as u32),
                Ordering::Equal => unsafe { unreachable_unchecked() },
            }
        }
    }

    pub fn clear(&mut self) {
        self.d = Box::new([0; 64])
    }

    pub fn write(&mut self, b: u8, mut x: usize, mut y: usize) -> bool {
        let b = if !self.hi_res {
            x *= 2;
            y *= 2;
            expand(b)
        } else {
            b as u16
        };

        let x = x % 128;
        let y = y % 64;

        let mut erased = false;
        self.dirty = true;
        let mut b = b as u128;
        b = b.rotate_left(112 - x as u32);

        if b & self.d[y] != 0 {
            erased = true
        };
        self.d[y] ^= b;

        if !self.hi_res {
            if b & self.d[y + 1] != 0 {
                erased = true
            };
            self.d[y + 1] ^= b;
        }

        erased
    }

    pub fn read(&mut self) -> &[u128; 64] {
        self.dirty = false;
        &self.d
    }

    pub fn read_px(&mut self, x: usize, y: usize) -> bool {
        self.dirty = false;
        let (shifted, _) = self.d[y].overflowing_shr(127 - x as u32);
        (shifted & 1) == 1
    }

    pub fn hi_res(&self) -> bool {
        self.hi_res
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for Display {
    fn default() -> Self {
        Display::new()
    }
}

pub const DEFAULT_FONT: [u8; 240] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xe0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0x80, // C
    0xF0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    // Super CHIP fonts
    0xFF, 0xFF, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xFF, 0xFF, // 0
    0x18, 0x78, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0xFF, 0xFF, // 1
    0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, // 2
    0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, // 3
    0xC3, 0xC3, 0xC3, 0xC3, 0xFF, 0xFF, 0x03, 0x03, 0x03, 0x03, // 4
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, // 5
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, // 6
    0xFF, 0xFF, 0x03, 0x03, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x18, // 7
    0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, // 8
    0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, // 9
    0x7E, 0xFF, 0xC3, 0xC3, 0xC3, 0xFF, 0xFF, 0xC3, 0xC3, 0xC3, // A
    0xFC, 0xFC, 0xC3, 0xC3, 0xFC, 0xFC, 0xC3, 0xC3, 0xFC, 0xFC, // B
    0x3C, 0xFF, 0xC3, 0xC0, 0xC0, 0xC0, 0xC0, 0xC3, 0xFF, 0x3C, // C
    0xFC, 0xFE, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xFE, 0xFC, // D
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, // E
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0xC0, 0xC0, 0xC0, 0xC0, // F
];
