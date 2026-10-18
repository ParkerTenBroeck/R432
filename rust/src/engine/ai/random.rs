use crate::engine::{Engine, Move, board::PieceColor};

use super::xorshift;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Random;

impl Random {
    pub fn choose_move(
        &mut self,
        engine: &Engine,
        color: PieceColor,
        random: u32,
    ) -> Option<Move> {
        let mut selected = None;
        let mut best_score = i32::MIN;
        let mut tie_breaker = random;

        for &from in engine.pieces(color) {
            for kind in engine.moves(from).moves_iter() {
                tie_breaker = xorshift(tie_breaker);
                let score = tie_breaker as i32;
                if score > best_score || (score == best_score && tie_breaker & 1 != 0) {
                    best_score = score;
                    selected = Some(Move::new(from, kind));
                }
            }
        }

        selected
    }
}
