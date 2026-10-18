use crate::engine::{
    Engine, BoardMove, MoveKind, Move,
    board::{Cell, Idx, PieceColor, Tile},
    moves::KNIGHT_DIRS,
};

use super::xorshift;

const DIRECT_CHECK_BONUS: i32 = 200;
const OPENING_COUNT: usize = 8;
const ENDGAME_PIECE_LIMIT: u8 = 14;
const ENDGAME_OPPONENT_LIMIT: u8 = 4;
const WINNING_MOVE_SCORE: i32 = 8_000;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum OpeningLine {
    KingPawn,
    QueenPawn,
    CFilePawn,
    Reti,
    Spanish,
    London,
    FourKnights,
    Fianchetto,
    Finished = u8::MAX - 1,
    Unselected = u8::MAX,
}

impl OpeningLine {
    fn random(random: u32) -> Self {
        match random & 7 {
            0 => Self::KingPawn,
            1 => Self::QueenPawn,
            2 => Self::CFilePawn,
            3 => Self::Reti,
            4 => Self::Spanish,
            5 => Self::London,
            6 => Self::FourKnights,
            7 => Self::Fianchetto,
            _ => unreachable!(),
        }
    }

    fn book_index(self) -> Option<usize> {
        match self {
            Self::KingPawn => Some(0),
            Self::QueenPawn => Some(1),
            Self::CFilePawn => Some(2),
            Self::Reti => Some(3),
            Self::Spanish => Some(4),
            Self::London => Some(5),
            Self::FourKnights => Some(6),
            Self::Fianchetto => Some(7),
            Self::Finished | Self::Unselected => None,
        }
    }

    fn next(self) -> Self {
        Self::random(self as u32 + 1)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Greedy {
    opening: OpeningLine,
}

impl Greedy {
    pub const fn new() -> Self {
        Self {
            opening: OpeningLine::Unselected,
        }
    }

    pub fn choose_move(
        &mut self,
        engine: &Engine,
        color: PieceColor,
        random: u32,
    ) -> Option<Move> {
        let first_move = engine.play() < 2;
        if self.opening == OpeningLine::Unselected {
            self.opening = if first_move {
                OpeningLine::random(random)
            } else {
                OpeningLine::Finished
            };
        }
        if self.opening != OpeningLine::Finished {
            let first_opening = self.opening;
            loop {
                if let Some(mov) = Self::opening_move(engine, color, self.opening) {
                    return Some(mov);
                }
                if !first_move {
                    break;
                }

                self.opening = self.opening.next();
                if self.opening == first_opening {
                    break;
                }
            }
            self.opening = OpeningLine::Finished;
        }

        let mut selected = None;
        let mut best_score = i32::MIN;
        let mut tie_breaker = random;
        let endgame = engine.piece_count(PieceColor::White)
            + engine.piece_count(PieceColor::Black)
            <= ENDGAME_PIECE_LIMIT
            || engine.piece_count(color.opposite()) <= ENDGAME_OPPONENT_LIMIT;

        for &from in engine.pieces(color) {
            let tile = engine.tile_at(from);
            for kind in engine.moves(from).moves_iter() {
                tie_breaker = xorshift(tie_breaker);
                let score = Self::score_move(engine, from, kind, tile, color, endgame);
                if score >= WINNING_MOVE_SCORE {
                    return Some(Move::new(from, kind));
                }
                if score > best_score || (score == best_score && tie_breaker & 1 != 0) {
                    best_score = score;
                    selected = Some(Move::new(from, kind));
                }
            }
        }

        selected
    }

    fn opening_move(
        engine: &Engine,
        color: PieceColor,
        opening: OpeningLine,
    ) -> Option<Move> {
        // Any material loss, check, or mismatch with the expected position is
        // enough evidence that blindly continuing the line is a bad idea.
        if engine.is_in_check(color) || engine.piece_count(color) < 16 {
            return None;
        }
        let line = OPENING_LINE_INDICIES.get(opening.book_index()?)?;
        let side_ply = engine.play() as usize / 2;
        if side_ply >= line.len as usize {
            return None;
        }
        let mut mov = OPENING_MOVES.get(line.start as usize + side_ply).copied()?;
        if color == PieceColor::Black {
            mov = BoardMove::new(Self::mirror(mov.from), Self::mirror(mov.to));
        }
        let from = mov.from;
        if engine.tile_at(from).color() != Some(color) || !engine.tile_at(mov.to).is_empty() {
            return None;
        }
        let mov = engine.find_move(from, mov.to)?;
        let to = mov.destination();
        let opponent = color.opposite();
        // Do not follow the book when it hangs the moved piece for free.
        if engine.will_cell_be_attacked(from, to, opponent)
            && !engine.will_cell_be_attacked(from, to, color)
        {
            return None;
        }

        Some(mov)
    }

    #[inline(always)]
    fn mirror(cell: Cell) -> Cell {
        Cell::from_index(cell.index() ^ 0b111_000)
    }

    fn score_move(
        engine: &Engine,
        from: Cell,
        mov: MoveKind,
        tile: Tile,
        color: PieceColor,
        endgame: bool,
    ) -> i32 {
        let to = mov.destination(from);
        let captured = match mov {
            MoveKind::Take(to) => engine.tile_at(to),
            MoveKind::Ep(_) => match color {
                PieceColor::White => Tile::BlackPawn,
                PieceColor::Black => Tile::WhitePawn,
            },
            _ => Tile::Empty,
        };
        let mut score = Self::piece_value(captured) * 10;
        let moved = Engine::promoted_tile(tile, to);
        if moved != tile {
            score += (Self::piece_value(moved) - Self::piece_value(tile)) * 10;
        }
        if matches!(mov, MoveKind::Castle(_)) {
            score += 150;
        }

        let x = to.x() as i32;
        let y = to.y() as i32;
        let centrality = 14 - (x * 2 - 7).abs() - (y * 2 - 7).abs();
        let home_rank = match color {
            PieceColor::White => Idx::I7,
            PieceColor::Black => Idx::I0,
        };
        match tile {
            Tile::WhitePawn => {
                score += (from.y() as i32 - y) * if endgame { 12 } else { 5 };
                score += i32::from(matches!(to.x(), Idx::I3 | Idx::I4)) * 8;
            }
            Tile::BlackPawn => {
                score += (y - from.y() as i32) * if endgame { 12 } else { 5 };
                score += i32::from(matches!(to.x(), Idx::I3 | Idx::I4)) * 8;
            }
            Tile::WhiteKnight | Tile::BlackKnight => {
                score += centrality * 3;
                score += i32::from(from.y() == home_rank) * 18;
            }
            Tile::WhiteBishop | Tile::BlackBishop => {
                score += centrality;
                score += i32::from(from.y() == home_rank) * 14;
            }
            Tile::WhiteQueen | Tile::BlackQueen => {
                score += centrality / 2;
                score -= i32::from(from.y() == home_rank) * 20;
            }
            Tile::WhiteKing | Tile::BlackKing => {
                if endgame {
                    let enemy = engine.king(color.opposite());
                    let old_distance = from.x().abs_diff(enemy.x()) + from.y().abs_diff(enemy.y());
                    let new_distance = to.x().abs_diff(enemy.x()) + to.y().abs_diff(enemy.y());
                    score += centrality * 2 + (old_distance as i32 - new_distance as i32) * 14;
                } else {
                    score -= centrality * 2;
                }
            }
            _ => {}
        }

        if endgame && engine.move_gives_check(from, mov, color) {
            score += DIRECT_CHECK_BONUS;
        }

        if moved != Tile::king(color) {
            let opponent = color.opposite();
            let attacked = engine.is_attacked_by_pawn(from, to, opponent)
                || engine.is_attacked_by_steps(from, to, &KNIGHT_DIRS, Tile::knight(opponent))
                || (!captured.is_empty() && engine.will_cell_be_attacked(from, to, opponent));
            if attacked {
                score -= Self::piece_value(moved) * 8;
            }
        }

        for (age, previous) in engine.recent_moves(color).iter().enumerate() {
            if previous.reverses(from, to) {
                score -= 120;
            }
            if previous.ends_at(from) {
                score -= 48 >> age;
            }
        }

        score
    }

    fn piece_value(tile: Tile) -> i32 {
        match tile {
            Tile::WhitePawn | Tile::BlackPawn => 100,
            Tile::WhiteKnight | Tile::BlackKnight => 320,
            Tile::WhiteBishop | Tile::BlackBishop => 330,
            Tile::WhiteRook | Tile::BlackRook => 500,
            Tile::WhiteQueen | Tile::BlackQueen => 900,
            _ => 0,
        }
    }
}

impl Default for Greedy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct BookLine {
    start: u8,
    len: u8,
}

impl BookLine {
    const fn new(start: u8, len: u8) -> Self {
        Self { start, len }
    }
}

const OPENING_LINE_INDICIES: [BookLine; OPENING_COUNT] = [
    BookLine::new(0, 6),
    BookLine::new(6, 5),
    BookLine::new(11, 4),
    BookLine::new(15, 6),
    BookLine::new(21, 5),
    BookLine::new(26, 5),
    BookLine::new(31, 5),
    BookLine::new(36, 5),
];

// expressed from White's perspective.
const OPENING_MOVES: [BoardMove; 41] = [
    // Italian
    BoardMove::new(Cell::new_const(4, 6), Cell::new_const(4, 4)),
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(5, 7), Cell::new_const(2, 4)),
    BoardMove::new(Cell::new_const(3, 6), Cell::new_const(3, 5)),
    BoardMove::new(Cell::new_const(4, 7), Cell::new_const(6, 7)),
    BoardMove::new(Cell::new_const(2, 6), Cell::new_const(2, 5)),
    // Queen's Gambit
    BoardMove::new(Cell::new_const(3, 6), Cell::new_const(3, 4)),
    BoardMove::new(Cell::new_const(2, 6), Cell::new_const(2, 4)),
    BoardMove::new(Cell::new_const(1, 7), Cell::new_const(2, 5)),
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(4, 6), Cell::new_const(4, 5)),
    // English
    BoardMove::new(Cell::new_const(2, 6), Cell::new_const(2, 4)),
    BoardMove::new(Cell::new_const(1, 7), Cell::new_const(2, 5)),
    BoardMove::new(Cell::new_const(6, 6), Cell::new_const(6, 5)),
    BoardMove::new(Cell::new_const(5, 7), Cell::new_const(6, 6)),
    // Reti
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(6, 6), Cell::new_const(6, 5)),
    BoardMove::new(Cell::new_const(5, 7), Cell::new_const(6, 6)),
    BoardMove::new(Cell::new_const(4, 7), Cell::new_const(6, 7)),
    BoardMove::new(Cell::new_const(3, 6), Cell::new_const(3, 5)),
    BoardMove::new(Cell::new_const(1, 7), Cell::new_const(3, 6)),
    // Spanish
    BoardMove::new(Cell::new_const(4, 6), Cell::new_const(4, 4)),
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(5, 7), Cell::new_const(1, 3)),
    BoardMove::new(Cell::new_const(3, 6), Cell::new_const(3, 5)),
    BoardMove::new(Cell::new_const(4, 7), Cell::new_const(6, 7)),
    // London
    BoardMove::new(Cell::new_const(3, 6), Cell::new_const(3, 4)),
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(2, 7), Cell::new_const(5, 4)),
    BoardMove::new(Cell::new_const(4, 6), Cell::new_const(4, 5)),
    BoardMove::new(Cell::new_const(2, 6), Cell::new_const(2, 5)),
    // Four Knights
    BoardMove::new(Cell::new_const(4, 6), Cell::new_const(4, 4)),
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(1, 7), Cell::new_const(2, 5)),
    BoardMove::new(Cell::new_const(5, 7), Cell::new_const(1, 3)),
    BoardMove::new(Cell::new_const(4, 7), Cell::new_const(6, 7)),
    // King's Fianchetto
    BoardMove::new(Cell::new_const(6, 6), Cell::new_const(6, 5)),
    BoardMove::new(Cell::new_const(5, 7), Cell::new_const(6, 6)),
    BoardMove::new(Cell::new_const(6, 7), Cell::new_const(5, 5)),
    BoardMove::new(Cell::new_const(3, 6), Cell::new_const(3, 5)),
    BoardMove::new(Cell::new_const(4, 7), Cell::new_const(6, 7)),
];