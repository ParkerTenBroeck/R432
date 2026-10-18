#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum Tile {
    Empty = 0x0,

    WhitePawn = 0x1,
    WhiteBishop = 0x2,
    WhiteKnight = 0x3,
    WhiteRook = 0x4,
    WhiteQueen = 0x5,
    WhiteKing = 0x6,

    BlackPawn = 0x7,
    BlackBishop = 0x8,
    BlackKnight = 0x9,
    BlackRook = 0xA,
    BlackQueen = 0xB,
    BlackKing = 0xC,

    Move = 0xD,
    X = 0xE,
    None2 = 0xF,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PieceColor {
    White = 0,
    Black = 1,
}

impl PieceColor {
    #[inline(always)]
    pub const fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    pub const fn pawn_dir(self) -> i8 {
        match self {
            Self::White => -1,
            Self::Black => 1,
        }
    }

    pub const fn pawn_start_y(self) -> Idx {
        match self {
            Self::White => Idx::I6,
            Self::Black => Idx::I1,
        }
    }
}

impl Tile {
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        matches!(self, Tile::Empty)
    }

    #[inline(always)]
    pub const fn color(self) -> Option<PieceColor> {
        match self {
            Tile::WhitePawn
            | Tile::WhiteBishop
            | Tile::WhiteKnight
            | Tile::WhiteRook
            | Tile::WhiteQueen
            | Tile::WhiteKing => Some(PieceColor::White),
            Tile::BlackPawn
            | Tile::BlackBishop
            | Tile::BlackKnight
            | Tile::BlackRook
            | Tile::BlackQueen
            | Tile::BlackKing => Some(PieceColor::Black),
            _ => None,
        }
    }

    pub fn is_enemy(self, color: PieceColor) -> bool {
        self.color() == Some(color.opposite())
    }

    pub const fn bishop(color: PieceColor) -> Self {
        match color {
            PieceColor::White => Tile::WhiteBishop,
            PieceColor::Black => Tile::BlackBishop,
        }
    }

    pub const fn knight(color: PieceColor) -> Self {
        match color {
            PieceColor::White => Tile::WhiteKnight,
            PieceColor::Black => Tile::BlackKnight,
        }
    }

    pub const fn rook(color: PieceColor) -> Self {
        match color {
            PieceColor::White => Tile::WhiteRook,
            PieceColor::Black => Tile::BlackRook,
        }
    }

    pub const fn queen(color: PieceColor) -> Self {
        match color {
            PieceColor::White => Tile::WhiteQueen,
            PieceColor::Black => Tile::BlackQueen,
        }
    }

    pub const fn king(color: PieceColor) -> Self {
        match color {
            PieceColor::White => Tile::WhiteKing,
            PieceColor::Black => Tile::BlackKing,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct BoardMove {
    pub from: Cell,
    pub to: Cell,
}

impl BoardMove {
    pub const fn new(from: Cell, to: Cell) -> Self {
        Self { from, to }
    }

    pub fn reverses(self, from: Cell, to: Cell) -> bool {
        self == Self::new(to, from)
    }

    pub fn ends_at(self, cell: Cell) -> bool {
        self.to == cell
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Board {
    board: [u32; 8],
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Cell(u8);

impl Cell {
    #[inline(always)]
    pub fn new(x: impl Into<Idx>, y: impl Into<Idx>) -> Self {
        Self((y.into() as u8) << 3 | x.into() as u8)
    }

    #[inline(always)]
    pub const fn new_const(x: u8, y: u8) -> Self {
        Self((y & 0b111) << 3 | (x & 0b111))
    }

    #[inline(always)]
    pub const fn x(self) -> Idx {
        // SAFETY: this will always be 0-7
        unsafe { core::mem::transmute::<u8, Idx>(self.0 & 0b111) }
    }

    #[inline(always)]
    pub const fn y(self) -> Idx {
        // SAFETY: this will always be 0-7
        unsafe { core::mem::transmute::<u8, Idx>(self.0 >> 3) }
    }

    pub fn offset(self, dx: i8, dy: i8) -> Option<Self> {
        let x = self.x() as i8 + dx;
        let y = self.y() as i8 + dy;

        if (0..8).contains(&x) && (0..8).contains(&y) {
            Some(Self::new(x as u8, y as u8))
        } else {
            None
        }
    }

    #[inline(always)]
    pub const fn index(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub const fn from_index(index: u8) -> Self {
        Self(index & 0b111111)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Idx {
    I0 = 0,
    I1 = 1,
    I2 = 2,
    I3 = 3,
    I4 = 4,
    I5 = 5,
    I6 = 6,
    I7 = 7,
}

impl Idx {
    #[inline(always)]
    pub const fn abs_diff(&self, other: Idx) -> u8 {
        (*self as u8).abs_diff(other as u8)
    }
}

impl From<u8> for Idx {
    fn from(value: u8) -> Self {
        match value & 0b111 {
            0 => Self::I0,
            1 => Self::I1,
            2 => Self::I2,
            3 => Self::I3,
            4 => Self::I4,
            5 => Self::I5,
            6 => Self::I6,
            7 => Self::I7,
            _ => unreachable!(),
        }
    }
}

impl Default for Board {
    #[inline]
    #[rustfmt::skip]
    fn default() -> Self {
        Self {
            board: [
                0xA98CB89A,
                0x77777777,
                0x00000000,
                0x00000000,
                0x00000000,
                0x00000000,
                0x11111111,
                0x43265234,
            ],
        }
    }
}

impl Board {
    #[inline(always)]
    pub fn get(&self, x: impl Into<Idx>, y: impl Into<Idx>) -> Tile {
        let raw = (self.board[y.into() as usize] >> (x.into() as u8 * 4)) & 0xF;
        // SAFETY: Tile exhaustively represents every four-bit value.
        unsafe { core::mem::transmute(raw as u8) }
    }

    #[inline(always)]
    pub fn set(&mut self, x: impl Into<Idx>, y: impl Into<Idx>, tile: Tile) {
        let y = y.into() as usize;
        let x = x.into() as u8;
        // The compiler *should* know that these are always in bounds... 
        self.board[y] &= !(0xF << (x * 4));
        self.board[y] |= (tile as u32) << (x * 4);
    }
}
