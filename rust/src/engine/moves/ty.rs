use crate::engine::board::Cell;

#[derive(Clone, Copy)]
pub struct Move {
    from: Cell,
    kind: MoveKind,
}

impl Move {
    pub const fn new(from: Cell, kind: MoveKind) -> Self {
        Self { from, kind }
    }

    pub const fn from(self) -> Cell {
        self.from
    }

    pub const fn kind(self) -> MoveKind {
        self.kind
    }

    pub const fn destination(self) -> Cell {
        self.kind.destination(self.from)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CastleKind {
    KingSide = 0,
    QueenSide = 1
}

#[repr(C, u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveKind {
    Move(Cell),
    Take(Cell),
    Ep(Cell),
    Castle(CastleKind),
}

impl MoveKind {
    #[inline(always)]
    pub const fn destination(self, from: Cell) -> Cell {
        match self {
            Self::Move(to) | Self::Take(to) | Self::Ep(to) => to,
            Self::Castle(CastleKind::KingSide) => Cell::new_const(6, from.y() as u8),
            Self::Castle(CastleKind::QueenSide) => Cell::new_const(2, from.y() as u8),
        }
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PackedMoveKind(u8);

impl PackedMoveKind {
    #[inline(always)]
    #[allow(clippy::identity_op)]
    pub const fn new(mov: MoveKind) -> Self {
        let (id, to) = match mov {
            MoveKind::Move(to) => (0, to.index()),
            MoveKind::Take(to) => (1, to.index()),
            MoveKind::Ep(to) => (2, to.index()),
            MoveKind::Castle(kind) => (3, kind as u8),
        };
        PackedMoveKind((id << 6) | to)
    }

    #[inline(always)]
    pub const fn kind(self) -> MoveKind {
        let repr = [self.0 >> 6, self.0 & 0b111111];
        // SAFETY: MoveKind is repr C, varient is before data. 
        // all 4 varients are covered with the top 2 bits
        // the data for Move, Take, Ep is valid for all 6 bits
        // the data for castle. is only valid for 0, and 1. but since we only construct self.0 in new. we know it is only ever 0 or 1
        unsafe {
            core::mem::transmute::<[u8; 2], MoveKind>(repr)
        }
    }
}


#[cfg(test)]
mod test {
    use crate::engine::moves::{CastleKind, MoveKind, PackedMoveKind, board::Cell};
  
    #[test]
    fn packed_move_kind() {
        fn assert_encoding(mov: MoveKind) {
            assert_eq!(mov, PackedMoveKind::new(mov).kind())
        }

        assert_encoding(MoveKind::Castle(CastleKind::KingSide));
        assert_encoding(MoveKind::Castle(CastleKind::QueenSide));
        for i in 0..=0b111111 {
            assert_encoding(MoveKind::Move(Cell::from_index(i)));
            assert_encoding(MoveKind::Take(Cell::from_index(i)));
            assert_encoding(MoveKind::Ep(Cell::from_index(i)));
        }
    }
}
