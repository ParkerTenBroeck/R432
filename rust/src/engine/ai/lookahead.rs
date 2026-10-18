use crate::engine::{
    Engine, MoveKind, Move,
    board::{PieceColor, Tile},
};

use super::xorshift;

const MATE_SCORE: i32 = 30_000;
const SEARCH_BOUND: i32 = 32_000;
const CHECK_PENALTY: i32 = 25;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Lookahead {
    depth: u8,
}

impl Lookahead {
    pub const fn new(depth: u8) -> Self {
        Self { depth }
    }

    pub const fn depth(self) -> u8 {
        self.depth
    }

    pub fn choose_move(&mut self, engine: &Engine, color: PieceColor, random: u32) -> Option<Move> {
        let mut best = None;
        let mut alpha = -SEARCH_BOUND;
        let mut candidates = Vec::new();

        for &from in engine.pieces(color) {
            for kind in engine.moves(from).moves_iter() {
                candidates.push(Move::new(from, kind));
            }
        }
        Self::shuffle(&mut candidates, random);

        for candidate in candidates {
            let mut next = *engine;
            next.apply_move(candidate);
            let score = -Self::search(
                &next,
                color.opposite(),
                self.depth.saturating_sub(1),
                -SEARCH_BOUND,
                -alpha,
            );
            if best.is_none() || score > alpha {
                alpha = score;
                best = Some(candidate);
            }
        }

        best
    }

    fn shuffle(moves: &mut [Move], mut random: u32) {
        for index in (1..moves.len()).rev() {
            random = xorshift(random);
            let selected = random as usize % (index + 1);
            moves.swap(index, selected);
        }
    }

    fn search(engine: &Engine, color: PieceColor, depth: u8, mut alpha: i32, beta: i32) -> i32 {
        if depth == 0 {
            return if engine.has_legal_move(color) {
                Self::evaluate(engine, color)
            } else {
                Self::terminal_score(engine, color, depth)
            };
        }

        let mut found_move = false;
        for captures in [true, false] {
            for &from in engine.pieces(color) {
                for kind in engine.moves(from).moves_iter() {
                    if Self::is_capture(kind) != captures {
                        continue;
                    }

                    found_move = true;
                    let mut next = *engine;
                    next.apply_move(Move::new(from, kind));
                    let score = -Self::search(&next, color.opposite(), depth - 1, -beta, -alpha);
                    if score >= beta {
                        return beta;
                    }
                    alpha = alpha.max(score);
                }
            }
        }

        if found_move {
            alpha
        } else {
            Self::terminal_score(engine, color, depth)
        }
    }

    fn terminal_score(engine: &Engine, color: PieceColor, depth: u8) -> i32 {
        if engine.is_in_check(color) {
            -MATE_SCORE - i32::from(depth)
        } else {
            0
        }
    }

    fn evaluate(engine: &Engine, color: PieceColor) -> i32 {
        let material = Self::material(engine, color) - Self::material(engine, color.opposite());
        material - i32::from(engine.is_in_check(color)) * CHECK_PENALTY
            + i32::from(engine.is_in_check(color.opposite())) * CHECK_PENALTY
    }

    fn material(engine: &Engine, color: PieceColor) -> i32 {
        engine
            .pieces(color)
            .iter()
            .map(|&cell| Self::piece_value(engine.tile_at(cell)))
            .sum()
    }

    const fn piece_value(tile: Tile) -> i32 {
        match tile {
            Tile::WhitePawn | Tile::BlackPawn => 100,
            Tile::WhiteKnight | Tile::BlackKnight => 320,
            Tile::WhiteBishop | Tile::BlackBishop => 330,
            Tile::WhiteRook | Tile::BlackRook => 500,
            Tile::WhiteQueen | Tile::BlackQueen => 900,
            _ => 0,
        }
    }

    const fn is_capture(mov: MoveKind) -> bool {
        matches!(mov, MoveKind::Take(_) | MoveKind::Ep(_))
    }
}
