use core::mem::MaybeUninit;

use super::*;


#[derive(Clone, Copy)]
#[repr(align(4))]
pub struct MoveList<const L: usize = 28> {
    moves: [MaybeUninit<PackedMoveKind>; L],
    len: u8,
    from: Cell,
}

impl<const L: usize> core::ops::Deref for MoveList<L> {
    type Target = [PackedMoveKind];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: len never exceeds L, and entries below len are initialized.
        unsafe {
            self.moves
                .get_unchecked(..self.len as usize)
                .assume_init_ref()
        }
    }
}

impl<const L: usize> core::ops::DerefMut for MoveList<L> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: len never exceeds L, and entries below len are initialized.
        unsafe {
            self.moves
                .get_unchecked_mut(..self.len as usize)
                .assume_init_mut()
        }
    }
}

impl<const L: usize> MoveList<L> {
    pub const fn new(from: Cell) -> Self {
        Self {
            moves: [MaybeUninit::uninit(); L],
            len: 0,
            from,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[allow(unused)]
    pub const fn from(&self) -> Cell {
        self.from
    }

    pub fn push(&mut self, mov: MoveKind) -> Result<(), ()> {
        let packed = PackedMoveKind::new(mov);

        let len = const {
            assert!(L <= 255);
            L as u8
        };
        if self.len < len {
            // SAFETY: write is always in bounds of array
            unsafe {
                self.moves
                    .get_unchecked_mut(self.len as usize)
                    .write(packed)
            };
            self.len += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    #[inline(always)]
    pub fn moves_iter(&self) -> impl Iterator<Item = MoveKind> {
        self.iter().copied().map(PackedMoveKind::kind)
    }
}