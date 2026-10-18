mod apply;
pub mod list;
mod ty;


use crate::engine::moves::list::MoveList;

use super::*;
pub use ty::*;


#[derive(Clone, Copy)]
enum PinAxis {
    Horizontal,
    Vertical,
    DiagonalDown,
    DiagonalUp,
}

#[derive(Clone, Copy)]
struct HypotheticalMove {
    from: Cell,
    to: Cell,
    kind: MoveKind,
    moved: Tile,
    color: PieceColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    IllegalMove,
}

pub const BISHOP_DIRS: [(i8, i8); 4] = [(-1, -1), (1, -1), (-1, 1), (1, 1)];
pub const ROOK_DIRS: [(i8, i8); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];
pub const QUEEN_DIRS: [(i8, i8); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
pub const KNIGHT_DIRS: [(i8, i8); 8] = [
    (-1, -2),
    (1, -2),
    (-2, -1),
    (2, -1),
    (-2, 1),
    (2, 1),
    (-1, 2),
    (1, 2),
];
pub const KING_DIRS: [(i8, i8); 8] = QUEEN_DIRS;

impl Engine {
    #[inline(never)]
    pub fn has_legal_move(&self, color: PieceColor) -> bool {
        let side = self.side(color);
        for &cell in &side.pieces {
            if !self.moves(cell).is_empty() {
                return true;
            }
        }
        false
    }

    pub fn find_move(&self, from: Cell, to: Cell) -> Option<Move> {
        let moves = self.moves(from);
        moves.moves_iter()
            .find(|mov| mov.destination(from) == to)
                .map(|kind| Move::new(from, kind))
    }

    #[inline(always)]
    pub fn is_in_check(&self, color: PieceColor) -> bool {
        self.side(color).in_check
    }

    fn check_evasion(&self, color: PieceColor) -> CheckEvasion {
        self.side(color).evasion
    }

    pub fn validate_move(&self, from: Cell, to: Cell) -> Result<Move, MoveError> {
        self.find_move(from, to)
            .ok_or(MoveError::IllegalMove)
    }

    pub fn moves(&self, cell: Cell) -> MoveList {
        let mut moves = MoveList::new(cell);
        let tile = self.tile_at(cell);
        let Some(color) = tile.color() else {
            return moves;
        };

        if tile == Tile::king(color) {
            self.pseudo_moves(cell, tile, &mut moves);
            return moves;
        }

        let pin = self.pin_axis(cell, color);
        let evasion = if self.is_in_check(color) {
            self.check_evasion(color)
        } else {
            CheckEvasion::None
        };

        if pin.is_none() && matches!(evasion, CheckEvasion::None) {
            self.pseudo_moves(cell, tile, &mut moves);
            return moves;
        }

        let mut pseudo = MoveList::new(cell);
        self.pseudo_moves(cell, tile, &mut pseudo);
        for mov in pseudo.iter().copied().map(PackedMoveKind::kind) {
            let stays_pinned = pin.is_none_or(|axis| Self::move_stays_on_axis(cell, &mov, axis));
            if stays_pinned && self.move_resolves_check(cell, &mov, color, evasion) {
                _ = moves.push(mov);
            }
        }
        moves
    }

    fn move_resolves_check(
        &self,
        from: Cell,
        mov: &MoveKind,
        color: PieceColor,
        evasion: CheckEvasion,
    ) -> bool {
        let CheckEvasion::Single { checker, ray } = evasion else {
            return matches!(evasion, CheckEvasion::None);
        };

        let captured = match *mov {
            MoveKind::Take(to) => Some(to),
            MoveKind::Ep(to) => Some(Cell::new(to.x(), from.y())),
            _ => None,
        };
        if captured == Some(checker) {
            return true;
        }

        let Some((dx, dy)) = ray else {
            return false;
        };
        let king = self.side(color).king;
        let to = mov.destination(from);
        let mut scan = king;
        while let Some(next) = scan.offset(dx, dy) {
            if next == checker {
                return false;
            }
            if next == to {
                return true;
            }
            scan = next;
        }
        false
    }

    fn pseudo_moves(&self, cell: Cell, tile: Tile, moves: &mut MoveList) {
        match tile {
            Tile::WhitePawn => self.pawn_moves(cell, PieceColor::White, moves),
            Tile::BlackPawn => self.pawn_moves(cell, PieceColor::Black, moves),

            Tile::WhiteBishop => self.ray_moves(cell, PieceColor::White, &BISHOP_DIRS, moves),
            Tile::BlackBishop => self.ray_moves(cell, PieceColor::Black, &BISHOP_DIRS, moves),

            Tile::WhiteKnight => self.knight_moves(cell, PieceColor::White, moves),
            Tile::BlackKnight => self.knight_moves(cell, PieceColor::Black, moves),

            Tile::WhiteRook => self.ray_moves(cell, PieceColor::White, &ROOK_DIRS, moves),
            Tile::BlackRook => self.ray_moves(cell, PieceColor::Black, &ROOK_DIRS, moves),

            Tile::WhiteQueen => self.ray_moves(cell, PieceColor::White, &QUEEN_DIRS, moves),
            Tile::BlackQueen => self.ray_moves(cell, PieceColor::Black, &QUEEN_DIRS, moves),

            Tile::WhiteKing => self.king_moves(cell, PieceColor::White, moves),
            Tile::BlackKing => self.king_moves(cell, PieceColor::Black, moves),
            _ => {}
        }
    }

    fn pin_axis(&self, cell: Cell, color: PieceColor) -> Option<PinAxis> {
        let king = self.side(color).king;
        let delta_x = cell.x() as i8 - king.x() as i8;
        let delta_y = cell.y() as i8 - king.y() as i8;
        let (dx, dy, axis) = if delta_x == 0 {
            (0, delta_y.signum(), PinAxis::Vertical)
        } else if delta_y == 0 {
            (delta_x.signum(), 0, PinAxis::Horizontal)
        } else if delta_x.abs() == delta_y.abs() {
            let dx = delta_x.signum();
            let dy = delta_y.signum();
            let axis = if dx == dy {
                PinAxis::DiagonalDown
            } else {
                PinAxis::DiagonalUp
            };
            (dx, dy, axis)
        } else {
            return None;
        };

        let mut scan = king;
        while let Some(next) = scan.offset(dx, dy) {
            if next == cell {
                break;
            }
            if !self.tile_at(next).is_empty() {
                return None;
            }
            scan = next;
        }

        let attacker = color.opposite();
        scan = cell;
        while let Some(next) = scan.offset(dx, dy) {
            let tile = self.tile_at(next);
            if tile.is_empty() {
                scan = next;
                continue;
            }

            let pinner = match axis {
                PinAxis::Horizontal | PinAxis::Vertical => {
                    tile == Tile::rook(attacker) || tile == Tile::queen(attacker)
                }
                PinAxis::DiagonalDown | PinAxis::DiagonalUp => {
                    tile == Tile::bishop(attacker) || tile == Tile::queen(attacker)
                }
            };
            return pinner.then_some(axis);
        }

        None
    }

    fn move_stays_on_axis(from: Cell, mov: &MoveKind, axis: PinAxis) -> bool {
        let to = match *mov {
            MoveKind::Move(to) | MoveKind::Take(to) | MoveKind::Ep(to) => to,
            MoveKind::Castle(_) => return false,
        };

        let move_x = to.x() as i8 - from.x() as i8;
        let move_y = to.y() as i8 - from.y() as i8;
        match axis {
            PinAxis::Horizontal => move_y == 0,
            PinAxis::Vertical => move_x == 0,
            PinAxis::DiagonalDown => move_x == move_y,
            PinAxis::DiagonalUp => move_x == -move_y,
        }
    }

    fn ep_keeps_rank_safe(&self, from: Cell, to: Cell, color: PieceColor) -> bool {
        let captured = Cell::new(to.x(), from.y());
        let left = self.first_piece_on_rank_after_ep(from, -1, captured);
        let right = self.first_piece_on_rank_after_ep(from, 1, captured);
        let king = Tile::king(color);
        let rook = Tile::rook(color.opposite());
        let queen = Tile::queen(color.opposite());

        !((left == king && (right == rook || right == queen))
            || (right == king && (left == rook || left == queen)))
    }

    fn first_piece_on_rank_after_ep(&self, from: Cell, dx: i8, captured: Cell) -> Tile {
        let mut scan = from;
        while let Some(next) = scan.offset(dx, 0) {
            if next == captured {
                scan = next;
                continue;
            }

            let tile = self.tile_at(next);
            if !tile.is_empty() {
                return tile;
            }
            scan = next;
        }
        Tile::Empty
    }

    fn pawn_moves(&self, cell: Cell, color: PieceColor, moves: &mut MoveList) {
        let dir = color.pawn_dir();
        let start_y = color.pawn_start_y();
        let x = cell.x() as u8;
        let y = cell.y();

        if let Some(one) = cell.offset(0, dir)
            && self.tile_at(one).is_empty()
        {
            _ = moves.push(MoveKind::Move(one));

            if y == start_y
                && let Some(two) = cell.offset(0, dir * 2)
                && self.tile_at(two).is_empty()
            {
                _ = moves.push(MoveKind::Move(two));
            }
        }

        for dx in [-1, 1] {
            if let Some(to) = cell.offset(dx, dir) {
                let tile = self.tile_at(to);
                if tile.is_enemy(color) {
                    _ = moves.push(MoveKind::Take(to));
                }
            }
        }

        if let Some(ep) = self.ep {
            let ep_x = ep.x() as u8;
            let ep_y = ep.y();
            if ep_y == y
                && x.abs_diff(ep_x) == 1
                && self.tile_at(ep).is_enemy(color)
                && let Some(to) = ep.offset(0, dir)
                && self.ep_keeps_rank_safe(cell, to, color)
            {
                _ = moves.push(MoveKind::Ep(to));
            }
        }
    }

    fn knight_moves(&self, cell: Cell, color: PieceColor, moves: &mut MoveList) {
        for (dx, dy) in KNIGHT_DIRS {
            if let Some(to) = cell.offset(dx, dy) {
                let tile = self.tile_at(to);
                if tile.is_empty() {
                    _ = moves.push(MoveKind::Move(to));
                } else if tile.is_enemy(color) {
                    _ = moves.push(MoveKind::Take(to));
                }
            }
        }
    }

    fn ray_moves(&self, cell: Cell, color: PieceColor, dirs: &[(i8, i8)], moves: &mut MoveList) {
        for &(dx, dy) in dirs {
            let mut from = cell;

            while let Some(to) = from.offset(dx, dy) {
                let tile = self.tile_at(to);
                if tile.is_empty() {
                    _ = moves.push(MoveKind::Move(to));
                    from = to;
                } else {
                    if tile.is_enemy(color) {
                        _ = moves.push(MoveKind::Take(to));
                    }
                    break;
                }
            }
        }
    }

    fn king_moves(&self, cell: Cell, color: PieceColor, moves: &mut MoveList) {
        for &(dx, dy) in &KING_DIRS {
            if let Some(to) = cell.offset(dx, dy) {
                let tile = self.tile_at(to);

                if tile.color() == Some(color) {
                    continue;
                }

                if self.will_cell_be_attacked(cell, to, color.opposite()) {
                    continue;
                }

                if tile.is_empty() {
                    _ = moves.push(MoveKind::Move(to));
                } else {
                    _ = moves.push(MoveKind::Take(to));
                }
            }
        }

        self.castle_moves(color, moves);
    }

    fn castle_moves(&self, color: PieceColor, moves: &mut MoveList) {
        let (y, king, rook) = match color {
            PieceColor::White => (7, Tile::WhiteKing, Tile::WhiteRook),
            PieceColor::Black => (0, Tile::BlackKing, Tile::BlackRook),
        };
        let castle = &self.side(color).castle;

        if self.board.get(4, y) != king {
            return;
        }

        let king_cell = Cell::new(4, y);

        if castle.kingside
            && self.board.get(5, y).is_empty()
            && self.board.get(6, y).is_empty()
            && self.board.get(7, y) == rook
            && !self.will_cell_be_attacked(king_cell, king_cell, color.opposite())
            && !self.will_cell_be_attacked(king_cell, Cell::new(5, y), color.opposite())
            && !self.will_cell_be_attacked(king_cell, Cell::new(6, y), color.opposite())
        {
            _ = moves.push(MoveKind::Castle(CastleKind::KingSide));
        }

        if castle.queenside
            && self.board.get(1, y).is_empty()
            && self.board.get(2, y).is_empty()
            && self.board.get(3, y).is_empty()
            && self.board.get(0, y) == rook
            && !self.will_cell_be_attacked(king_cell, king_cell, color.opposite())
            && !self.will_cell_be_attacked(king_cell, Cell::new(3, y), color.opposite())
            && !self.will_cell_be_attacked(king_cell, Cell::new(2, y), color.opposite())
        {
            _ = moves.push(MoveKind::Castle(CastleKind::QueenSide));
        }
    }

    pub fn will_cell_be_attacked(
        &self,
        from: Cell,
        to: Cell,
        attacker: PieceColor,
    ) -> bool {
        self.is_attacked_by_pawn(from, to, attacker)
            || self.is_attacked_by_steps(from, to, &KNIGHT_DIRS, Tile::knight(attacker))
            || self.is_attacked_by_steps(from, to, &KING_DIRS, Tile::king(attacker))
            || self.is_attacked_by_rays(
                from,
                to,
                &BISHOP_DIRS,
                Tile::bishop(attacker),
                Tile::queen(attacker),
            )
            || self.is_attacked_by_rays(
                from,
                to,
                &ROOK_DIRS,
                Tile::rook(attacker),
                Tile::queen(attacker),
            )
    }

    pub fn move_gives_check(
        &self,
        from: Cell,
        mov: MoveKind,
        color: PieceColor,
    ) -> bool {
        let to = mov.destination(from);
        let moved = Self::promoted_tile(self.tile_at(from), to);
        let king = self.king(color.opposite());
        let position = HypotheticalMove {
            from,
            to,
            kind: mov,
            moved,
            color,
        };
        let dx = king.x() as i8 - to.x() as i8;
        let dy = king.y() as i8 - to.y() as i8;
        let direct = match moved {
            Tile::WhitePawn | Tile::BlackPawn => {
                dx.abs() == 1 && dy == color.pawn_dir()
            }
            Tile::WhiteKnight | Tile::BlackKnight => {
                matches!((dx.abs(), dy.abs()), (1, 2) | (2, 1))
            }
            Tile::WhiteKing | Tile::BlackKing => dx.abs() <= 1 && dy.abs() <= 1,
            Tile::WhiteBishop | Tile::BlackBishop if dx.abs() == dy.abs() => {
                self.check_ray_after_move(from, to, king, dx.signum(), dy.signum())
            }
            Tile::WhiteRook | Tile::BlackRook if dx == 0 || dy == 0 => {
                self.check_ray_after_move(from, to, king, dx.signum(), dy.signum())
            }
            Tile::WhiteQueen | Tile::BlackQueen
                if dx == 0 || dy == 0 || dx.abs() == dy.abs() =>
            {
                self.check_ray_after_move(from, to, king, dx.signum(), dy.signum())
            }
            _ => false,
        };
        if direct || self.discovered_check_after_move(king, from, position) {
            return true;
        }

        match mov {
            MoveKind::Ep(_) => self.discovered_check_after_move(
                king,
                Cell::new(to.x(), from.y()),
                position,
            ),
            MoveKind::Castle(kind) => {
                let (rook_from, rook_to) = match kind {
                    CastleKind::KingSide => (7, 5),
                    CastleKind::QueenSide => (0, 3),
                };
                self.discovered_check_after_move(
                    king,
                    Cell::new(rook_from, from.y()),
                    position,
                ) || self.discovered_check_after_move(
                    king,
                    Cell::new(rook_to, from.y()),
                    position,
                )
            }
            _ => false,
        }
    }

    fn check_ray_after_move(
        &self,
        from: Cell,
        to: Cell,
        king: Cell,
        dx: i8,
        dy: i8,
    ) -> bool {
        let mut scan = to;
        while let Some(next) = scan.offset(dx, dy) {
            if next == king {
                return true;
            }
            if next != from && !self.tile_at(next).is_empty() {
                return false;
            }
            scan = next;
        }
        false
    }

    fn discovered_check_after_move(
        &self,
        king: Cell,
        through: Cell,
        position: HypotheticalMove,
    ) -> bool {
        let dx = through.x() as i8 - king.x() as i8;
        let dy = through.y() as i8 - king.y() as i8;
        if (dx == 0 && dy == 0) || (dx != 0 && dy != 0 && dx.abs() != dy.abs()) {
            return false;
        }
        let step_x = dx.signum();
        let step_y = dy.signum();
        let slider = if step_x == 0 || step_y == 0 {
            Tile::rook(position.color)
        } else {
            Tile::bishop(position.color)
        };
        let ep_capture = match position.kind {
            MoveKind::Ep(_) => Some(Cell::new(position.to.x(), position.from.y())),
            _ => None,
        };
        let rook = match position.kind {
            MoveKind::Castle(CastleKind::KingSide) => {
                Some((Cell::new(7, position.from.y()), Cell::new(5, position.from.y())))
            }
            MoveKind::Castle(CastleKind::QueenSide) => {
                Some((Cell::new(0, position.from.y()), Cell::new(3, position.from.y())))
            }
            _ => None,
        };

        let mut scan = king;
        while let Some(next) = scan.offset(step_x, step_y) {
            let tile = if next == position.from
                || ep_capture == Some(next)
                || rook.is_some_and(|(rook_from, _)| next == rook_from)
            {
                Tile::Empty
            } else if next == position.to {
                position.moved
            } else if rook.is_some_and(|(_, rook_to)| next == rook_to) {
                Tile::rook(position.color)
            } else {
                self.tile_at(next)
            };
            if tile.is_empty() {
                scan = next;
                continue;
            }
            return tile == slider || tile == Tile::queen(position.color);
        }
        false
    }

    pub fn is_attacked_by_pawn(&self, from: Cell, to: Cell, attacker: PieceColor) -> bool {
        let (dy, pawn) = match attacker {
            PieceColor::White => (1, Tile::WhitePawn),
            PieceColor::Black => (-1, Tile::BlackPawn),
        };

        for dx in [-1, 1] {
            if let Some(attacker_cell) = to.offset(dx, dy)
                && self.tile_at_after_move(attacker_cell, from, to) == pawn
            {
                return true;
            }
        }

        false
    }

    pub fn is_attacked_by_steps(
        &self,
        from: Cell,
        to: Cell,
        dirs: &[(i8, i8)],
        piece: Tile,
    ) -> bool {
        for &(dx, dy) in dirs {
            if let Some(attacker_cell) = to.offset(dx, dy)
                && self.tile_at_after_move(attacker_cell, from, to) == piece
            {
                return true;
            }
        }

        false
    }

    pub fn is_attacked_by_rays(
        &self,
        from: Cell,
        to: Cell,
        dirs: &[(i8, i8)],
        piece: Tile,
        queen: Tile,
    ) -> bool {
        for &(dx, dy) in dirs {
            let mut scan = to;

            while let Some(next) = scan.offset(dx, dy) {
                let tile = self.tile_at_after_move(next, from, to);
                if tile.is_empty() {
                    scan = next;
                    continue;
                }

                if tile == piece || tile == queen {
                    return true;
                }

                break;
            }
        }

        false
    }

    pub(crate) fn tile_at_after_move(&self, cell: Cell, from: Cell, to: Cell) -> Tile {
        if cell == from || cell == to {
            Tile::Empty
        } else {
            self.tile_at(cell)
        }
    }
}