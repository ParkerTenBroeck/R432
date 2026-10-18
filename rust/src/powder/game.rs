use crate::engine::{
    Engine, Game as EngineGame, GameInterface, GameOutcome, Move, MoveKind, ai::{Greedy, Random}, board::{Cell, Idx, PieceColor, Tile}, moves::CastleKind,
};

use super::mmio::{
    self,
    term::{CellColor, Color, CursorPos, ScrollprintFlags},
};

const BLACK: Color = Color::C0;
const GREY: Color = Color::C1;
const WHITE: Color = Color::C2;

const LIGHT_BG: Color = Color::C3;
const DARK_BG: Color = Color::C4;

const LIGHT_BG_HIGH: Color = Color::C5;
const DARK_BG_HIGH: Color = Color::C6;

const GREEN: Color = Color::C7;
const RED: Color = Color::C8;
const BLUE: Color = Color::C9;

pub const PAWN_CHAR: u8 = 0x10;
pub const BISHOP_CHAR: u8 = 0x11;
pub const KNIGHT_CHAR: u8 = 0x12;
pub const ROOK_CHAR: u8 = 0x13;
pub const QUEEN_CHAR: u8 = 0x14;
pub const KING_CHAR: u8 = 0x15;
pub const CIRCLE_CHAR: u8 = 0x16;
pub const X_CHAR: u8 = b'x';

enum BgStyle {
    Regular,
    Highlight,
}

enum PieceStyle {
    Regular,
    Take,
    Highlight,
    Selected,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Controller {
    Player,
    Greedy(Greedy),
    Random(Random),
}

struct NoLegalMove;

#[derive(Clone, Copy)]
#[repr(Rust, packed(4))]
struct HighlightedMoves {
    cells: u64,
}

impl HighlightedMoves {
    fn new() -> Self {
        Self { cells: 0 }
    }

    fn set(&mut self, cell: Cell) {
        self.cells |= 1 << cell.index();
    }

    fn contains(&self, cell: Cell) -> bool {
        self.cells & (1 << cell.index()) != 0
    }
}

pub struct Game;

impl Game {
    pub fn new() -> Self {
        Self
    }

    #[inline(never)]
    fn draw_tile(&self, cell: Cell, bs: BgStyle, ps: PieceStyle, tile: Tile) {
        let x = cell.x() as u8;
        let y = cell.y() as u8;

        let piece_color = if tile.is_empty() {
            LIGHT_BG_HIGH
        } else {
            match ps {
                PieceStyle::Regular if tile.color() == Some(PieceColor::Black) => BLACK,
                PieceStyle::Regular => WHITE,
                PieceStyle::Take => RED,
                PieceStyle::Highlight => BLUE,
                PieceStyle::Selected => GREEN,
            }
        };
        let tile_color = match bs {
            BgStyle::Regular if (x + y) % 2 == 1 => LIGHT_BG,
            BgStyle::Regular => DARK_BG,
            BgStyle::Highlight if (x + y) % 2 == 1 => LIGHT_BG_HIGH,
            BgStyle::Highlight => DARK_BG_HIGH,
        };
        let piece = match tile {
            Tile::Empty => b' ',
            Tile::Move => CIRCLE_CHAR,
            Tile::X => X_CHAR,
            Tile::BlackPawn | Tile::WhitePawn => PAWN_CHAR,
            Tile::BlackBishop | Tile::WhiteBishop => BISHOP_CHAR,
            Tile::BlackKnight | Tile::WhiteKnight => KNIGHT_CHAR,
            Tile::BlackRook | Tile::WhiteRook => ROOK_CHAR,
            Tile::BlackQueen | Tile::WhiteQueen => QUEEN_CHAR,
            Tile::BlackKing | Tile::WhiteKing => KING_CHAR,
            _ => b' ',
        };
        mmio::terminal.print_with_color_and_pos(
            ScrollprintFlags::default(),
            CellColor::new(piece_color, tile_color),
            CursorPos::new(x + 1, y + 1),
            piece,
        );
    }

    pub fn init_board(&self, engine: &Engine) {
        use super::printer::Printer;
        let mut printer = Printer::new(&mmio::terminal, 12, 10);
        printer.init();
        printer.set_color(WHITE, GREY);
        printer.clear();

        for i in 0..8 {
            mmio::terminal.print_with_color_and_pos(
                ScrollprintFlags::term_mode(),
                CellColor::new(WHITE, GREY),
                CursorPos::new(0, 1 + i),
                b'8' - i,
            );

            mmio::terminal.print_with_color_and_pos(
                ScrollprintFlags::term_mode(),
                CellColor::new(WHITE, GREY),
                CursorPos::new(9, 1 + i),
                b'8' - i,
            );
            mmio::terminal.print_with_color_and_pos(
                ScrollprintFlags::term_mode(),
                CellColor::new(WHITE, GREY),
                CursorPos::new(1 + i, 0),
                b'a' + i,
            );

            mmio::terminal.print_with_color_and_pos(
                ScrollprintFlags::term_mode(),
                CellColor::new(WHITE, GREY),
                CursorPos::new(1 + i, 9),
                b'a' + i,
            );
        }

        for x in 0..8 {
            for y in 0..8 {
                self.draw_tile(
                    Cell::new(x, y),
                    BgStyle::Regular,
                    PieceStyle::Regular,
                    engine.tile_at(Cell::new(x, y)),
                );
            }
        }
    }

    pub fn begin_play(self) {
        let white = Self::select_controller(PieceColor::White);
        let black = Self::select_controller(PieceColor::Black);
        let mut game = EngineGame::new(white, black, self);
        match game.play() {
            Ok(outcome) => game.interface_mut().show_outcome(outcome),
            Err(_) => game.interface_mut().show_result(RED, "Engine Bug"),
        }
    }

    fn select_controller(color: PieceColor) -> Controller {
        use super::printer::Printer;

        let mut printer = Printer::new(&mmio::terminal, 12, 10);
        printer.init();
        printer.set_color(WHITE, GREY);
        printer.clear();
        let (heading, heading_color) = match color {
            PieceColor::White => ("White", WHITE),
            PieceColor::Black => ("Black", BLACK),
        };
        printer.set_color(heading_color, GREY);
        printer.set_cursor(3, 2);
        printer.print(heading);

        printer.set_cursor(2, 4);
        printer.print("1 Player");
        printer.set_cursor(2, 6);
        printer.print("2 AI Greed");
        printer.set_cursor(2, 8);
        printer.print("3 AI Rand");

        mmio::terminal.reset_keyboard();
        loop {
            match mmio::terminal.read_blocking() as u8 {
                b'1' => return Controller::Player,
                b'2' => return Controller::Greedy(Greedy::new()),
                b'3' => return Controller::Random(Random),
                _ => {}
            }
        }
    }

    fn show_winner(&self, winner: PieceColor) {
        let (color, message) = match winner {
            PieceColor::White => (WHITE, "White wins"),
            PieceColor::Black => (BLACK, "Black wins"),
        };
        self.show_result(color, message);
    }

    fn show_outcome(&self, outcome: GameOutcome) {
        match outcome {
            GameOutcome::Checkmate { winner } => self.show_winner(winner),
            GameOutcome::Stalemate => self.show_result(RED, "Stalemate"),
            GameOutcome::FiftyMoveDraw => self.show_result(RED, "50 move draw"),
        }
    }

    #[inline(never)]
    fn show_result(&self, color: Color, message: &str) {
        use super::printer::Printer;

        let mut printer = Printer::new(&mmio::terminal, 12, 10);
        printer.init();
        printer.set_color(color, GREY);
        printer.clear();
        printer.set_cursor(((12 - message.len()) / 2) as u8, 5);
        printer.print(message);
    }

    #[inline(never)]
    fn draw_turn(&self, engine: &Engine) {
        match engine.turn() {
            PieceColor::White => {
                mmio::terminal.print_with_color_and_pos(
                    ScrollprintFlags::default(),
                    CellColor::new(WHITE, GREY),
                    CursorPos::new(11, 5),
                    b'W',
                );
                mmio::terminal.print_with_color_and_pos(
                    ScrollprintFlags::default(),
                    CellColor::new(BLACK, GREY),
                    CursorPos::new(11, 4),
                    b' ',
                );
            }
            PieceColor::Black => {
                mmio::terminal.print_with_color_and_pos(
                    ScrollprintFlags::default(),
                    CellColor::new(BLACK, GREY),
                    CursorPos::new(11, 4),
                    b'B',
                );
                mmio::terminal.print_with_color_and_pos(
                    ScrollprintFlags::default(),
                    CellColor::new(BLACK, GREY),
                    CursorPos::new(11, 5),
                    b' ',
                );
            }
        }

        mmio::terminal.print_with_color_and_pos(
            ScrollprintFlags::default(),
            CellColor::new(RED, GREY),
            CursorPos::new(11, 3),
            if engine.is_in_check(PieceColor::Black) {
                b'C'
            } else {
                b' '
            },
        );
        mmio::terminal.print_with_color_and_pos(
            ScrollprintFlags::default(),
            CellColor::new(RED, GREY),
            CursorPos::new(11, 6),
            if engine.is_in_check(PieceColor::White) {
                b'C'
            } else {
                b' '
            },
        );
    }

    pub fn player_move(&self, engine: &Engine, turn: PieceColor) -> Move {
        mmio::terminal.reset_keyboard();

        'select_piece: loop {
            let Some(from) = self.read_cell(engine, None) else {
                continue;
            };
            if engine.tile_at(from).color() != Some(turn) {
                self.flash_invalid(engine, from, None);
                continue;
            }

            let highlighted = self.highlight_moves(engine, from);
            loop {
                let selection = Some((from, highlighted));
                let Some(to) = self.read_cell(engine, selection) else {
                    self.unhighlight_moves(engine, from, highlighted);
                    continue 'select_piece;
                };
                let mov = match engine.validate_move(from, to) {
                    Ok(mov) => mov,
                    Err(_) => {
                        self.flash_invalid(engine, to, selection);
                        continue;
                    }
                };

                self.unhighlight_moves(engine, from, highlighted);
                return mov;
            }
        }
    }

    fn read_cell(
        &self,
        engine: &Engine,
        selection: Option<(Cell, HighlightedMoves)>,
    ) -> Option<Cell> {
        let mut x = None;
        let mut y = None;

        loop {
            let old_x = x;
            let old_y = y;
            let key = mmio::terminal.read_blocking() as u8;
            match key {
                b'1'..=b'8' => y = Some(Idx::from(b'8' - key)),
                b'a'..=b'h' => x = Some(Idx::from(key - b'a')),
                b'A'..=b'H' => x = Some(Idx::from(key - b'A')),
                8 | 127 => {
                    if x.is_some() || y.is_some() {
                        self.restore_partial_cell(engine, x, y, selection);
                        x = None;
                        y = None;
                        continue;
                    }
                    return None;
                }
                _ => continue,
            }

            self.restore_partial_cell(engine, old_x, old_y, selection);
            if let (Some(x), Some(y)) = (x, y) {
                return Some(Cell::new(x, y));
            }
            self.highlight_partial_cell(engine, x, y, selection);
        }
    }

    fn highlight_partial_cell(
        &self,
        engine: &Engine,
        x: Option<Idx>,
        y: Option<Idx>,
        selection: Option<(Cell, HighlightedMoves)>,
    ) {
        match (x, y) {
            (Some(x), None) => {
                for y in 0..8 {
                    self.highlight_cell(engine, Cell::new(x, y), selection);
                }
            }
            (None, Some(y)) => {
                for x in 0..8 {
                    self.highlight_cell(engine, Cell::new(x, y), selection);
                }
            }
            _ => {}
        }
    }

    fn highlight_cell(
        &self,
        engine: &Engine,
        cell: Cell,
        selection: Option<(Cell, HighlightedMoves)>,
    ) {
        let tile = engine.tile_at(cell);
        if let Some((from, highlighted)) = selection {
            if cell == from {
                self.draw_tile(cell, BgStyle::Regular, PieceStyle::Selected, tile);
                return;
            }
            if highlighted.contains(cell) {
                if tile.is_empty() {
                    self.draw_tile(cell, BgStyle::Highlight, PieceStyle::Highlight, Tile::Move);
                } else {
                    self.draw_tile(cell, BgStyle::Highlight, PieceStyle::Take, tile);
                }
                return;
            }
        }

        self.draw_tile(cell, BgStyle::Highlight, PieceStyle::Regular, tile);
    }

    fn restore_partial_cell(
        &self,
        engine: &Engine,
        x: Option<Idx>,
        y: Option<Idx>,
        selection: Option<(Cell, HighlightedMoves)>,
    ) {
        match (x, y) {
            (Some(x), None) => {
                for y in 0..8 {
                    self.restore_cell(engine, Cell::new(x, y), selection);
                }
            }
            (None, Some(y)) => {
                for x in 0..8 {
                    self.restore_cell(engine, Cell::new(x, y), selection);
                }
            }
            _ => {}
        }
    }

    fn restore_cell(
        &self,
        engine: &Engine,
        cell: Cell,
        selection: Option<(Cell, HighlightedMoves)>,
    ) {
        if let Some((from, highlighted)) = selection {
            if cell == from {
                self.draw_tile(
                    cell,
                    BgStyle::Regular,
                    PieceStyle::Selected,
                    engine.tile_at(cell),
                );
                return;
            }
            if highlighted.contains(cell) {
                let tile = engine.tile_at(cell);
                if tile.is_empty() {
                    self.draw_tile(cell, BgStyle::Regular, PieceStyle::Highlight, Tile::Move);
                } else {
                    self.draw_tile(cell, BgStyle::Regular, PieceStyle::Take, tile);
                }
                return;
            }
        }

        self.draw_tile(
            cell,
            BgStyle::Regular,
            PieceStyle::Regular,
            engine.tile_at(cell),
        );
    }

    fn highlight_moves(&self, engine: &Engine, from: Cell) -> HighlightedMoves {
        let mut highlighted = HighlightedMoves::new();
        self.draw_tile(
            from,
            BgStyle::Regular,
            PieceStyle::Selected,
            engine.tile_at(from),
        );
        for mov in engine.moves(from).moves_iter() {
            let to = match mov {
                MoveKind::Move(to) | MoveKind::Ep(to) => {
                    self.draw_tile(to, BgStyle::Regular, PieceStyle::Highlight, Tile::Move);
                    to
                }
                MoveKind::Take(to) => {
                    self.draw_tile(to, BgStyle::Regular, PieceStyle::Take, engine.tile_at(to));
                    to
                }
                MoveKind::Castle(CastleKind::KingSide) => {
                    let to = Cell::new(6, from.y());
                    self.draw_tile(to, BgStyle::Regular, PieceStyle::Highlight, Tile::Move);
                    to
                }
                MoveKind::Castle(CastleKind::QueenSide) => {
                    let to = Cell::new(2, from.y());
                    self.draw_tile(to, BgStyle::Regular, PieceStyle::Highlight, Tile::Move);
                    to
                }
            };
            highlighted.set(to);
        }
        highlighted
    }

    fn unhighlight_moves(&self, engine: &Engine, from: Cell, highlighted: HighlightedMoves) {
        self.draw_tile(
            from,
            BgStyle::Regular,
            PieceStyle::Regular,
            engine.tile_at(from),
        );
        for i in 0..64 {
            let to = Cell::from_index(i);
            if highlighted.contains(to) {
                self.draw_tile(
                    to,
                    BgStyle::Regular,
                    PieceStyle::Regular,
                    engine.tile_at(to),
                );
            }
        }
    }

    fn flash_invalid(
        &self,
        engine: &Engine,
        cell: Cell,
        selection: Option<(Cell, HighlightedMoves)>,
    ) {
        self.draw_tile(cell, BgStyle::Regular, PieceStyle::Take, Tile::X);
        super::time::sleep_for(30);
        self.restore_cell(engine, cell, selection);
    }

    fn redraw_move(&self, engine: &Engine, mov: Move) {
        let from = mov.from();
        let to = mov.destination();
        self.draw_tile(
            from,
            BgStyle::Regular,
            PieceStyle::Regular,
            engine.tile_at(from),
        );
        self.draw_tile(
            to,
            BgStyle::Regular,
            PieceStyle::Regular,
            engine.tile_at(to),
        );

        match mov.kind() {
            MoveKind::Ep(_) => {
                let captured = Cell::new(to.x(), from.y());
                self.draw_tile(
                    captured,
                    BgStyle::Regular,
                    PieceStyle::Regular,
                    engine.tile_at(captured),
                );
            }
            MoveKind::Castle(kind) => {
                let (rook_from, rook_to) = match kind {
                    CastleKind::KingSide => (Cell::new(7, from.y()), Cell::new(5, from.y())),
                    _ => (Cell::new(0, from.y()), Cell::new(3, from.y())),
                };
                for cell in [rook_from, rook_to] {
                    self.draw_tile(
                        cell,
                        BgStyle::Regular,
                        PieceStyle::Regular,
                        engine.tile_at(cell),
                    );
                }
            }
            MoveKind::Move(_) | MoveKind::Take(_) => {}
        }
    }
}

impl GameInterface<Controller> for Game {
    type Error = NoLegalMove;

    fn select_move(
        &mut self,
        controller: &mut Controller,
        engine: &Engine,
    ) -> Result<Move, Self::Error> {
        let color = engine.turn();
        match controller {
            Controller::Player => Ok(self.player_move(engine, color)),
            Controller::Greedy(ai) => ai
                .choose_move(engine, color, mmio::random_source.read())
                .ok_or(NoLegalMove),
            Controller::Random(ai) => ai
                .choose_move(engine, color, mmio::random_source.read())
                .ok_or(NoLegalMove),
        }
    }

    fn game_started(&mut self, engine: &Engine) {
        self.init_board(engine);
        self.draw_turn(engine);
    }

    fn turn_started(&mut self, engine: &Engine) {
        self.draw_turn(engine);
    }

    fn move_made(&mut self, engine: &Engine, mov: Move) {
        self.redraw_move(engine, mov);
    }
}
