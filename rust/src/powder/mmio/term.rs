use super::*;
use core::cell::UnsafeCell;

// R432 has one instruction stream and no interrupts, so this cannot be accessed concurrently.
struct KeyHistory(UnsafeCell<u8>);

unsafe impl Sync for KeyHistory {}

impl KeyHistory {
    const fn new() -> Self {
        Self(UnsafeCell::new(0))
    }

    #[inline(always)]
    fn get(&self) -> u8 {
        // SAFETY: the R432 has a single instruction stream and no interrupts.
        unsafe { *self.0.get() }
    }

    #[inline(always)]
    fn set(&self, key: u8) {
        // SAFETY: the R432 has a single instruction stream and no interrupts.
        unsafe { *self.0.get() = key };
    }
}

static LAST_BLOCKING_KEY: KeyHistory = KeyHistory::new();

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Range(u32);

impl Range {
    #[inline(always)]
    pub const fn new(first: u8, last: u8) -> Self {
        Self((first as u32) | ((last as u32) << 5))
    }

    #[inline(always)]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Color {
    #[default]
    C0,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    C8,
    C9,
    C10,
    C11,
    C12,
    C13,
    C14,
    C15,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct CellColor(u32);

impl CellColor {
    #[inline(always)]
    pub const fn new(foreground: Color, background: Color) -> Self {
        Self(foreground as u32 | ((background as u32) << 4))
    }

    #[inline(always)]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct CursorPos(u32);

impl CursorPos {
    #[inline(always)]
    pub const fn new(x: u8, y: u8) -> Self {
        Self(x as u32 | ((y as u32) << 5))
    }

    #[inline(always)]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct PixelPos {
    pub x: u8,
    pub y: u8,
}

#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Pixel {
    pub pos: PixelPos,
    pub color: Color,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct ScrollprintFlags(u8);

impl ScrollprintFlags {
    pub const POS_IN_DATA: Self = Self(1 << 6);
    pub const ENABLE_NLCHAR: Self = Self(1 << 5);
    pub const TERM_MODE_SCROLL: Self = Self(1 << 4);
    pub const ENABLE_SCROLLMASK: Self = Self(1 << 3);
    pub const COLUMN_ORIENTED: Self = Self(1 << 2);
    pub const COLOR_IN_DATA: Self = Self(1 << 1);
    pub const TERM_MODE: Self = Self(1 << 0);

    pub const SIMPLE_TERMINAL: Self =
        Self(Self::ENABLE_NLCHAR.bits() | Self::TERM_MODE_SCROLL.bits() | Self::TERM_MODE.bits());

    #[inline(always)]
    pub const fn pos_in_data() -> Self {
        Self::POS_IN_DATA
    }

    #[inline(always)]
    pub const fn enable_nlchar() -> Self {
        Self::ENABLE_NLCHAR
    }

    #[inline(always)]
    pub const fn term_mode_scroll() -> Self {
        Self::TERM_MODE_SCROLL
    }

    #[inline(always)]
    pub const fn enable_scrollmask() -> Self {
        Self::ENABLE_SCROLLMASK
    }

    #[inline(always)]
    pub const fn column_oriented() -> Self {
        Self::COLUMN_ORIENTED
    }

    #[inline(always)]
    pub const fn color_in_data() -> Self {
        Self::COLOR_IN_DATA
    }

    #[inline(always)]
    pub const fn term_mode() -> Self {
        Self::TERM_MODE
    }

    #[inline(always)]
    pub const fn simple_terminal() -> Self {
        Self::SIMPLE_TERMINAL
    }

    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

impl core::ops::BitOr for ScrollprintFlags {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.bits() | rhs.bits())
    }
}

impl core::ops::BitOrAssign for ScrollprintFlags {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.bits();
    }
}

#[repr(u8)]
pub enum CharMemSide {
    Top = 0,
    Bottom = 1,
}

// ----------------------------------------------

#[repr(C)]
pub struct Terminal {
    input: reg::R32,
    hrange: reg::W32<Range>,
    vrange: reg::W32<Range>,
    cursor: reg::W32<CursorPos>,
    nlchar: reg::W32,
    color: reg::W32<CellColor>,
    scrollmask: reg::W32,
    plotpix: reg::W32<Pixel>,
    padding_1: [reg::Pad32; 0x78],
    scrollprint: [reg::W32; 0x80],
    begin_bitmap: [reg::W32; 0x100],
    char_mem: [reg::W32; 0x200],
    end_bitmap: [reg::W32; 0x400],
}

impl Terminal {
    #[inline(always)]
    pub fn set_horizontal_range(&self, first: u8, last: u8) {
        self.hrange.write(Range::new(first, last));
    }

    #[inline(always)]
    pub fn set_vertical_range(&self, first: u8, last: u8) {
        self.vrange.write(Range::new(first, last));
    }

    #[inline(always)]
    pub fn set_cursor_position(&self, column: u8, row: u8) {
        self.cursor.write(CursorPos::new(column, row));
    }

    #[inline(always)]
    pub fn set_cell_color(&self, foreground: Color, background: Color) {
        self.color.write(CellColor::new(foreground, background));
    }

    #[inline(always)]
    pub fn set_newline_char(&self, value: u8) {
        self.nlchar.write(value as u32);
    }

    #[inline(always)]
    pub fn set_scrollmask(&self, value: u32) {
        self.scrollmask.write(value);
    }

    #[inline(always)]
    pub fn print_with_pos(&self, flags: ScrollprintFlags, pos: CursorPos, byte: u8) {
        self.scrollprint(
            flags | ScrollprintFlags::pos_in_data(),
            (pos.bits() << 16) | byte as u32,
        );
    }

    #[inline(always)]
    pub fn print_with_color(&self, flags: ScrollprintFlags, color: CellColor, byte: u8) {
        self.scrollprint(
            flags | ScrollprintFlags::color_in_data(),
            (color.bits() << 8) | byte as u32,
        );
    }

    #[inline(always)]
    pub fn print_with_color_and_pos(
        &self,
        flags: ScrollprintFlags,
        color: CellColor,
        pos: CursorPos,
        byte: u8,
    ) {
        self.scrollprint(
            flags | ScrollprintFlags::color_in_data() | ScrollprintFlags::pos_in_data(),
            (pos.bits() << 16) | (color.bits() << 8) | byte as u32,
        );
    }

    #[inline(always)]
    pub fn scrollprint(&self, flags: ScrollprintFlags, value: u32) {
        self.scrollprint[flags.bits() as usize].write(value);
    }

    #[inline(always)]
    pub fn print_byte(&self, flags: ScrollprintFlags, value: u8) {
        self.scrollprint(flags, value as u32);
    }

    #[inline(always)]
    pub fn plotpix(&self, pixel: Pixel) {
        self.plotpix.write(pixel);
    }

    #[inline(always)]
    pub fn set_char_mem(&self, char: u8, side: CharMemSide, bitmap: [u8; 4]) {
        self.char_mem[((char as usize) << 1) | side as usize].write(u32::from_ne_bytes(bitmap));
    }

    #[inline(always)]
    pub fn set_char_mem_whole(&self, char: u8, bitmap: [u8; 8]) {
        // kind of a stupid hack but whatever
        self.set_char_mem(char, CharMemSide::Top, (bitmap[0..4]).try_into().unwrap());
        self.set_char_mem(
            char,
            CharMemSide::Bottom,
            (bitmap[4..8]).try_into().unwrap(),
        );
    }

    #[inline(always)]
    pub fn read_keyboard(&self) -> u32 {
        self.input.read()
    }

    #[inline(always)]
    pub fn reset_keyboard(&self) {
        let _ = self.read_keyboard();
        LAST_BLOCKING_KEY.set(0);
    }

    #[inline(never)]
    pub fn read_blocking(&self) -> u32 {
        // A repeated key already waiting at function entry is stale bounce. Once polling begins,
        // the same key is accepted again so intentional repeated coordinates still work.
        let pending = self.read_keyboard();
        if pending & 0xFF != 0 && pending as u8 != LAST_BLOCKING_KEY.get() {
            LAST_BLOCKING_KEY.set(pending as u8);
            return pending;
        }

        loop {
            let got = self.read_keyboard();

            if got & 0xFF != 0 {
                LAST_BLOCKING_KEY.set(got as u8);
                return got;
            }
        }
    }
}
