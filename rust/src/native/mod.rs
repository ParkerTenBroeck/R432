use std::{
    env,
    io::{self, Write},
};

use crate::engine::{
    Engine, Game as EngineGame, GameInterface, GameOutcome, Move,
    ai::{Greedy, Lookahead, Random},
    board::{Cell, PieceColor, Tile},
    moves::MoveError,
};

mod bench;

const RESET: &str = "\x1b[0m";
const WHITE_FG: &str = "\x1b[1;38;2;255;248;220m";
const BLACK_FG: &str = "\x1b[1;38;2;24;24;28m";
const LABEL_FG: &str = "\x1b[38;2;170;180;190m";
const ALERT_FG: &str = "\x1b[1;38;2;255;72;72m";
const LIGHT_BG: &str = "\x1b[48;2;205;181;145m";
const DARK_BG: &str = "\x1b[48;2;181;136;99m";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Controller {
    Player,
    Greedy(Greedy),
    Random(Random),
    Lookahead(Lookahead),
}

impl Controller {
    fn parse(value: &str) -> Option<Self> {
        let value = value.to_ascii_lowercase();
        match value.as_str() {
            "player" => Some(Self::Player),
            "ai-greed" => Some(Self::Greedy(Greedy::new())),
            "ai-rand" => Some(Self::Random(Random)),
            _ => value
                .strip_prefix("ai-lookahead-")
                .and_then(|depth| depth.parse().ok())
                .filter(|depth| (1..=15).contains(depth))
                .map(|depth| Self::Lookahead(Lookahead::new(depth))),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::Greedy(_) => "Greedy AI",
            Self::Random(_) => "Random AI",
            Self::Lookahead(_) => "Lookahead AI",
        }
    }

    fn choose_move(&mut self, engine: &Engine) -> Option<Move> {
        let color = engine.turn();
        match self {
            Self::Player => None,
            Self::Greedy(ai) => ai.choose_move(engine, color, rand::random()),
            Self::Random(ai) => ai.choose_move(engine, color, rand::random()),
            Self::Lookahead(ai) => ai.choose_move(engine, color, rand::random()),
        }
    }
}

enum TurnError {
    Io(io::Error),
    Quit,
    NoLegalMove(PieceColor),
}

impl From<io::Error> for TurnError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

struct NativeUi;

impl GameInterface<Controller> for NativeUi {
    type Error = TurnError;

    fn select_move(
        &mut self,
        controller: &mut Controller,
        engine: &Engine,
    ) -> Result<Move, Self::Error> {
        let color = engine.turn();
        match controller {
            Controller::Player => self.read_player_move(engine, color),
            _ => controller
                .choose_move(engine)
                .ok_or(TurnError::NoLegalMove(color)),
        }
    }

    fn game_started(&mut self, engine: &Engine) {
        self.print_board(engine);
    }

    fn turn_started(&mut self, engine: &Engine) {
        let turn = engine.turn();
        if engine.is_in_check(turn) {
            println!(
                "{ALERT_FG}Check:{RESET} {}{}{} is in check.\n",
                color_foreground(turn),
                color_name(turn),
                RESET,
            );
        }
    }

    fn move_made(&mut self, engine: &Engine, mov: Move) {
        self.print_move(engine.turn(), mov, engine.play());
        self.print_board(engine);
    }
}

impl NativeUi {
    fn show_outcome(&self, outcome: GameOutcome) {
        match outcome {
            GameOutcome::Checkmate { winner } => println!(
                "{}{}{} wins by checkmate.",
                color_foreground(winner),
                color_name(winner),
                RESET,
            ),
            GameOutcome::Stalemate => println!("{ALERT_FG}Draw by stalemate.{RESET}"),
            GameOutcome::FiftyMoveDraw => {
                println!("{ALERT_FG}Draw by the fifty-move rule.{RESET}")
            }
        }
    }

    fn read_player_move(
        &mut self,
        engine: &Engine,
        color: PieceColor,
    ) -> Result<Move, TurnError> {
        fn parse_move(input: &str) -> Option<(Cell, Cell)> {
            let bytes = input.as_bytes();
            if bytes.len() != 4 {
                return None;
            }
            Some((parse_square(&bytes[..2])?, parse_square(&bytes[2..])?))
        }

        fn parse_square(value: &[u8]) -> Option<Cell> {
            let file = value[0].to_ascii_lowercase();
            let rank = value[1];
            if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
                return None;
            }
            Some(Cell::new(file - b'a', b'8' - rank))
        }

        let mut input = String::new();
        loop {
            print!(
                "{}{}{} move (e2e4, or quit): ",
                color_foreground(color),
                color_name(color),
                RESET,
            );
            io::stdout().flush()?;

            input.clear();
            if io::stdin().read_line(&mut input)? == 0 {
                return Err(TurnError::Quit);
            }
            let input = input.trim();
            if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                return Err(TurnError::Quit);
            }

            let Some((from, to)) = parse_move(input) else {
                println!("Enter a move as four characters, for example e2e4.");
                continue;
            };
            if engine.tile_at(from).color() != Some(color) {
                println!("Select one of {}'s pieces.", color_name(color));
                continue;
            }

            match engine.validate_move(from, to) {
                Ok(mov) => return Ok(mov),
                Err(MoveError::IllegalMove) => println!("That move is not legal."),
            }
        }
    }

    fn print_move(&self, color: PieceColor, mov: Move, ply: u16) {
        println!(
            "{}. {}{}{}: {}{}",
            ply.div_ceil(2),
            color_foreground(color),
            color_name(color),
            RESET,
            square_name(mov.from()),
            square_name(mov.destination()),
        );
    }

    fn print_board(&self, engine: &Engine) {
        fn piece_char(tile: Tile) -> char {
            match tile {
                Tile::WhitePawn => '♟',
                Tile::WhiteBishop => '♝',
                Tile::WhiteKnight => '♞',
                Tile::WhiteRook => '♜',
                Tile::WhiteQueen => '♛',
                Tile::WhiteKing => '♚',
                Tile::BlackPawn => '♟',
                Tile::BlackBishop => '♝',
                Tile::BlackKnight => '♞',
                Tile::BlackRook => '♜',
                Tile::BlackQueen => '♛',
                Tile::BlackKing => '♚',
                _ => ' ',
            }
        }

        print!("{LABEL_FG}   ");
        for file in 'a'..='h' {
            print!(" {file} ");
        }
        println!("{RESET}");

        for y in 0..8 {
            print!("{LABEL_FG} {} {RESET}", 8 - y);
            for x in 0..8 {
                let tile = engine.tile_at(Cell::new(x, y));
                let background = if (x + y) & 1 == 0 { LIGHT_BG } else { DARK_BG };
                let foreground = tile.color().map_or(LABEL_FG, color_foreground);
                print!("{background}{foreground} {} {RESET}", piece_char(tile));
            }
            println!("{LABEL_FG} {}{RESET}", 8 - y);
        }

        print!("{LABEL_FG}   ");
        for file in 'a'..='h' {
            print!(" {file} ");
        }
        println!("{RESET}\n");
    }
}

pub fn main() {
    if env::args().nth(1).as_deref() == Some("bench") {
        bench::run(env::args().skip(2));
        return;
    }

    let Some([white, black]) = parse_controllers() else {
        return;
    };

    println!(
        "{WHITE_FG}White {}{RESET} vs {BLACK_FG}Black {}{RESET}\n",
        white.name(),
        black.name(),
    );

    let mut game = EngineGame::new(white, black, NativeUi);
    match game.play() {
        Ok(outcome) => game.interface_mut().show_outcome(outcome),
        Err(TurnError::Quit) => println!("Game ended."),
        Err(TurnError::Io(error)) => eprintln!("Failed to read input: {error}"),
        Err(TurnError::NoLegalMove(color)) => {
            eprintln!(
                "{} has no legal move in a playable position.",
                color_name(color)
            )
        }
    }
}

fn parse_controllers() -> Option<[Controller; 2]> {
    fn print_usage() {
        println!("Usage: native [WHITE_MODE] [BLACK_MODE]");
        println!("       native bench [GAMES_PER_MATCHUP]");
        println!("Modes: player, ai-rand, ai-greed, ai-lookahead-1 through ai-lookahead-15");
        println!("Example: cargo run-native -- player ai-lookahead-4");
        println!("         cargo run-native -- bench 500");
    }

    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        print_usage();
        return None;
    }
    if args.len() > 2 {
        eprintln!("Expected at most two controller modes.\n");
        print_usage();
        return None;
    }

    let mut controllers = [
        Controller::Greedy(Greedy::new()),
        Controller::Greedy(Greedy::new()),
    ];
    for (index, value) in args.iter().enumerate() {
        let Some(controller) = Controller::parse(value) else {
            eprintln!("Unknown controller mode: {value}\n");
            print_usage();
            return None;
        };
        controllers[index] = controller;
    }
    Some(controllers)
}

fn color_name(color: PieceColor) -> &'static str {
    match color {
        PieceColor::White => "White",
        PieceColor::Black => "Black",
    }
}

fn color_foreground(color: PieceColor) -> &'static str {
    match color {
        PieceColor::White => WHITE_FG,
        PieceColor::Black => BLACK_FG,
    }
}

fn square_name(cell: Cell) -> String {
    let file = (b'a' + cell.x() as u8) as char;
    let rank = (b'8' - cell.y() as u8) as char;
    format!("{file}{rank}")
}
