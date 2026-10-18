use super::board::{Cell, PieceColor};

#[derive(Clone, Copy)]
pub struct PieceList<const L: usize = 16> {
    cells: [Cell; L],
    len: u8,
}

impl<const L: usize> core::ops::Deref for PieceList<L> {
    type Target = [Cell];
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: len starts at L and can only decrease, so it never exceeds L.
        unsafe { self.cells.get_unchecked(..self.len as usize) }
    }
}

impl<const L: usize> core::ops::DerefMut for PieceList<L> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: len starts at L and can only decrease, so it never exceeds L.
        unsafe { self.cells.get_unchecked_mut(..self.len as usize) }
    }
}

impl<'a, const L: usize> IntoIterator for &'a PieceList<L> {
    type Item = &'a Cell;
    type IntoIter = core::slice::Iter<'a, Cell>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, const L: usize> IntoIterator for &'a mut PieceList<L> {
    type Item = &'a mut Cell;
    type IntoIter = core::slice::IterMut<'a, Cell>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<const L: usize> PieceList<L> {
    pub const fn from_array(cells: [Cell; L]) -> Self {
        assert!(L <= u8::MAX as usize);
        Self {
            cells,
            len: L as u8,
        }
    }

    #[inline(always)]
    pub const fn len(&self) -> u8 {
        self.len
    }

    pub fn relocate(&mut self, from: Cell, to: Cell) {
        if let Some(cell) = self.iter_mut().find(|cell| **cell == from) {
            *cell = to;
        }
    }

    pub fn remove(&mut self, target: Cell) {
        if let Some(index) = self.iter().position(|cell| *cell == target) {
            self.len -= 1;
            // SAFETY: `index` came from the live slice, and decrementing a
            // non-empty length leaves the former last element in the array.
            unsafe {
                let replacement = *self.cells.get_unchecked(self.len as usize);
                *self.cells.get_unchecked_mut(index) = replacement;
            }
        }
    }
}

impl PieceList {
    pub(super) const fn starting(color: PieceColor) -> Self {
        let first = match color {
            PieceColor::White => 48,
            PieceColor::Black => 0,
        };
        let mut cells = [Cell::from_index(0); 16];
        let mut index = 0;
        while index < cells.len() {
            cells[index] = Cell::from_index(first + index as u8);
            index += 1;
        }
        Self::from_array(cells)
    }
}
