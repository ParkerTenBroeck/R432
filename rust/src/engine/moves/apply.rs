use super::*;

impl Engine {

    /// Assumes the move is valid.
    pub fn apply_move(&mut self, mov: Move) {
        let from = mov.from();
        let kind = mov.kind();
        let tile = self.tile_at(from);
        let to = mov.destination();
        // a validated move always starts on a piece. but this avoids a panic or unsafe
        let color = tile.color().unwrap_or(PieceColor::White);

        let pawn_move = matches!(tile, Tile::WhitePawn | Tile::BlackPawn);
        let double_pawn_move = pawn_move && from.y().abs_diff(to.y()) == 2;

        self.ply += 1;
        if pawn_move || matches!(kind, MoveKind::Take(_) | MoveKind::Ep(_)) {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock = self.halfmove_clock.saturating_add(1);
        }

        let side = self.side_mut(color);
        side.recent_moves[1] = side.recent_moves[0];
        side.recent_moves[0] = BoardMove::new(from, to);

        self.update_can_castle(from, to, tile);
        self.update_piece_lists(from, to, kind, color);
        self.apply_to_board(from, to, kind, tile, color);
        self.update_king_position(tile, to, color);
        self.ep = double_pawn_move.then_some(to);

        self.set_check_evasion(color, CheckEvasion::None);
        let opponent = color.opposite();
        let opponent_evasion = self.detect_check_evasion(opponent);
        self.set_check_evasion(opponent, opponent_evasion);
    }

    fn detect_check_evasion(&self, color: PieceColor) -> CheckEvasion {
        let king = self.side(color).king;
        let attacker = color.opposite();
        let mut first = None;
        let mut count = 0;
        let mut add = |checker, ray| {
            count += 1;
            if first.is_none() {
                first = Some((checker, ray));
            }
        };

        let (pawn_dy, pawn) = match attacker {
            PieceColor::White => (1, Tile::WhitePawn),
            PieceColor::Black => (-1, Tile::BlackPawn),
        };
        for dx in [-1, 1] {
            if let Some(cell) = king.offset(dx, pawn_dy)
                && self.tile_at(cell) == pawn
            {
                add(cell, None);
            }
        }

        for (dx, dy) in KNIGHT_DIRS {
            if let Some(cell) = king.offset(dx, dy)
                && self.tile_at(cell) == Tile::knight(attacker)
            {
                add(cell, None);
            }
        }

        for &(dx, dy) in &KING_DIRS {
            if let Some(cell) = king.offset(dx, dy)
                && self.tile_at(cell) == Tile::king(attacker)
            {
                add(cell, None);
            }
        }

        for &(dx, dy) in &QUEEN_DIRS {
            let mut scan = king;
            while let Some(next) = scan.offset(dx, dy) {
                let tile = self.tile_at(next);
                if tile.is_empty() {
                    scan = next;
                    continue;
                }

                let slider = if dx == 0 || dy == 0 {
                    tile == Tile::rook(attacker) || tile == Tile::queen(attacker)
                } else {
                    tile == Tile::bishop(attacker) || tile == Tile::queen(attacker)
                };
                if slider {
                    add(next, Some((dx, dy)));
                }
                break;
            }
        }

        match (count, first) {
            (0, _) => CheckEvasion::None,
            (1, Some((checker, ray))) => CheckEvasion::Single { checker, ray },
            _ => CheckEvasion::Double,
        }
    }
    

    fn apply_to_board(
        &mut self,
        from: Cell,
        to: Cell,
        mov: MoveKind,
        tile: Tile,
        color: PieceColor,
    ) {
        match mov {
            MoveKind::Move(_) | MoveKind::Take(_) => {
                self.move_tile(from, to, Self::promoted_tile(tile, to));
            }
            MoveKind::Ep(_) => {
                self.move_tile(from, to, tile);
                self.board.set(to.x(), from.y(), Tile::Empty);
            }
            MoveKind::Castle(CastleKind::KingSide) => self.apply_castle(from, to, tile, color, 7, 5),
            MoveKind::Castle(CastleKind::QueenSide) => self.apply_castle(from, to, tile, color, 0, 3),
        }
    }

    fn apply_castle(
        &mut self,
        from: Cell,
        to: Cell,
        king: Tile,
        color: PieceColor,
        rook_from_x: u8,
        rook_to_x: u8,
    ) {
        self.move_tile(from, to, king);
        self.move_tile(
            Cell::new(rook_from_x, from.y()),
            Cell::new(rook_to_x, from.y()),
            Tile::rook(color),
        );
    }

    fn update_king_position(&mut self, tile: Tile, to: Cell, color: PieceColor) {
        if tile == Tile::king(color) {
            self.side_mut(color).king = to;
        }
    }

    pub fn promoted_tile(tile: Tile, to: Cell) -> Tile {
        match (tile, to.y()) {
            (Tile::WhitePawn, Idx::I0) => Tile::WhiteQueen,
            (Tile::BlackPawn, Idx::I7) => Tile::BlackQueen,
            _ => tile,
        }
    }

    fn move_tile(&mut self, from: Cell, to: Cell, tile: Tile) {
        self.board.set(from.x(), from.y(), Tile::Empty);
        self.board.set(to.x(), to.y(), tile);
    }

    fn update_piece_lists(&mut self, from: Cell, to: Cell, mov: MoveKind, color: PieceColor) {
        self.side_mut(color).pieces.relocate(from, to);
        match mov {
            MoveKind::Take(_) => self.side_mut(color.opposite()).pieces.remove(to),
            MoveKind::Ep(_) => self
                .side_mut(color.opposite())
                .pieces
                .remove(Cell::new(to.x(), from.y())),
            MoveKind::Castle(CastleKind::KingSide) => self
                .side_mut(color)
                .pieces
                .relocate(Cell::new(7, from.y()), Cell::new(5, from.y())),
            MoveKind::Castle(CastleKind::QueenSide) => self
                .side_mut(color)
                .pieces
                .relocate(Cell::new(0, from.y()), Cell::new(3, from.y())),
            MoveKind::Move(_) => {}
        }
    }

    fn update_can_castle(&mut self, from: Cell, to: Cell, tile: Tile) {
        match tile {
            Tile::WhiteKing | Tile::BlackKing => {
                // this arm only matches colored king tiles. but this avoids a panic or unsafe
                self.side_mut(tile.color().unwrap_or(PieceColor::White))
                    .castle = Castle::none();
            }
            Tile::WhiteRook => self.update_rook_castle(PieceColor::White, from),
            Tile::BlackRook => self.update_rook_castle(PieceColor::Black, from),
            _ => {}
        }

        match self.tile_at(to) {
            Tile::WhiteRook => self.update_rook_castle(PieceColor::White, to),
            Tile::BlackRook => self.update_rook_castle(PieceColor::Black, to),
            _ => {}
        }
    }

    fn update_rook_castle(&mut self, color: PieceColor, cell: Cell) {
        let y = match color {
            PieceColor::White => Idx::I7,
            PieceColor::Black => Idx::I0,
        };
        let castle = &mut self.side_mut(color).castle;

        if cell.y() == y {
            match cell.x() {
                Idx::I0 => castle.queenside = false,
                Idx::I7 => castle.kingside = false,
                _ => {}
            }
        }
    }

    fn set_check_evasion(&mut self, color: PieceColor, evasion: CheckEvasion) {
        let side = self.side_mut(color);
        side.in_check = !matches!(evasion, CheckEvasion::None);
        side.evasion = evasion;
    }
}
