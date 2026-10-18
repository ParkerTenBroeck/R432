use crate::engine::board::{Board, BoardMove, Cell, Idx, PieceColor, Tile};
pub mod ai;
pub mod board;
mod game;
pub mod moves;
mod pieces;

pub use game::{Game, GameInterface, GameOutcome};
pub use moves::{MoveKind, Move};
use pieces::PieceList;

#[derive(Clone, Copy)]
pub enum CheckEvasion {
    None,
    Single {
        checker: Cell,
        ray: Option<(i8, i8)>,
    },
    Double,
}

/// set to false if king / rook moves.
#[derive(Clone, Copy)]
struct Castle {
    queenside: bool,
    kingside: bool,
}

impl Castle {
    fn none() -> Self {
        Self {
            queenside: false,
            kingside: false,
        }
    }
}

impl Default for Castle {
    fn default() -> Self {
        Self {
            queenside: true,
            kingside: true,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Engine {
    board: Board,
    /// is set to some if last move was a double pawn move (the location the pawn ended up)
    ep: Option<Cell>,
    sides: [SideState; 2],
    halfmove_clock: u8,
    ply: u16,
}

#[derive(Clone, Copy)]
struct SideState {
    castle: Castle,
    king: Cell,
    in_check: bool,
    evasion: CheckEvasion,
    recent_moves: [BoardMove; 2],
    pieces: PieceList,
}

impl SideState {
    fn new(king: Cell, color: PieceColor) -> Self {
        Self {
            castle: Castle::default(),
            king,
            in_check: false,
            evasion: CheckEvasion::None,
            recent_moves: [BoardMove::default(); 2],
            pieces: PieceList::starting(color),
        }
    }
}

impl Default for Engine {
    #[inline]
    fn default() -> Self {
        Self {
            board: Board::default(),
            ep: None,
            sides: [
                SideState::new(Cell::new(4, 7), PieceColor::White),
                SideState::new(Cell::new(4, 0), PieceColor::Black),
            ],
            halfmove_clock: 0,
            ply: 0,
        }
    }
}

impl Engine {
    fn side(&self, color: PieceColor) -> &SideState {
        &self.sides[color as usize]
    }

    fn side_mut(&mut self, color: PieceColor) -> &mut SideState {
        &mut self.sides[color as usize]
    }

    #[inline(always)]
    pub fn is_fifty_move_draw(&self) -> bool {
        self.halfmove_clock >= 100
    }

    #[inline(always)]
    pub fn tile_at(&self, cell: Cell) -> Tile {
        self.board.get(cell.x(), cell.y())
    }

    pub fn pieces(&self, color: PieceColor) -> &[Cell] {
        &self.side(color).pieces
    }

    pub fn piece_count(&self, color: PieceColor) -> u8 {
        self.side(color).pieces.len()
    }

    pub fn king(&self, color: PieceColor) -> Cell {
        self.side(color).king
    }

    pub fn recent_moves(&self, color: PieceColor) -> &[BoardMove; 2] {
        &self.side(color).recent_moves
    }

    pub const fn play(&self) -> u16 {
        self.ply
    }

    #[inline(always)]
    pub fn turn(&self) -> PieceColor {
        if self.play() & 1 == 0 {
            PieceColor::White
        } else {
            PieceColor::Black
        }
    }
}
