use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    thread,
    time::Instant,
};

use crate::engine::{
    Engine, Game as EngineGame, GameInterface, GameOutcome, Move,
    ai::{Greedy, Lookahead, Random},
    board::PieceColor,
};

use super::Controller;

const DEFAULT_GAMES: u32 = 500;
const AIS: [BenchAi; 5] = [
    BenchAi::Random,
    BenchAi::Greedy,
    BenchAi::Lookahead(1),
    BenchAi::Lookahead(2),
    BenchAi::Lookahead(3),
    // BenchAi::Lookahead(4),
];

#[derive(Clone, Copy)]
enum BenchAi {
    Random,
    Greedy,
    Lookahead(u8),
}

impl BenchAi {
    fn name(self) -> &'static str {
        match self {
            Self::Random => "ai-rand",
            Self::Greedy => "ai-greed",
            Self::Lookahead(1) => "ai-lookahead-1",
            Self::Lookahead(2) => "ai-lookahead-2",
            Self::Lookahead(3) => "ai-lookahead-3",
            Self::Lookahead(4) => "ai-lookahead-4",
            Self::Lookahead(_) => "ai-lookahead",
        }
    }

    fn controller(self) -> Controller {
        match self {
            Self::Random => Controller::Random(Random),
            Self::Greedy => Controller::Greedy(Greedy::new()),
            Self::Lookahead(depth) => Controller::Lookahead(Lookahead::new(depth)),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Record {
    wins: u32,
    losses: u32,
    draws: u32,
}

impl Record {
    fn games(self) -> u32 {
        self.wins + self.losses + self.draws
    }

    fn add(&mut self, result: ResultForWhite) {
        match result {
            ResultForWhite::Win => self.wins += 1,
            ResultForWhite::Loss => self.losses += 1,
            ResultForWhite::Draw => self.draws += 1,
        }
    }

    fn merge(&mut self, other: Self) {
        self.wins += other.wins;
        self.losses += other.losses;
        self.draws += other.draws;
    }
}

#[derive(Clone, Copy)]
enum ResultForWhite {
    Win,
    Loss,
    Draw,
}

struct QuietUi;

impl GameInterface<Controller> for QuietUi {
    type Error = ();

    fn select_move(
        &mut self,
        controller: &mut Controller,
        engine: &Engine,
    ) -> Result<Move, Self::Error> {
        controller.choose_move(engine).ok_or(())
    }
}

pub(super) fn run(mut args: impl Iterator<Item = String>) {
    let games = match args.next() {
        Some(value) => match value.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                eprintln!("GAMES_PER_MATCHUP must be a positive integer.");
                return;
            }
        },
        None => DEFAULT_GAMES,
    };
    if args.next().is_some() {
        eprintln!("Usage: native bench [GAMES_PER_MATCHUP]");
        return;
    }

    let games = games as usize;
    let Some(total_games) = games.checked_mul(AIS.len() * AIS.len()) else {
        eprintln!("The requested tournament is too large.");
        return;
    };
    let workers = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(total_games);

    println!(
        "Running {} games for each of {} ordered matchups ({} games total) on {} threads...",
        games,
        AIS.len() * AIS.len(),
        total_games,
        workers,
    );
    let _ = io::stdout().flush();

    let next_game = AtomicUsize::new(0);
    let completed_games = AtomicUsize::new(0);
    let last_report = AtomicU64::new(0);
    let failed = AtomicBool::new(false);
    let started = Instant::now();
    let worker_results = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            handles.push(scope.spawn(|| {
                let mut results = [[Record::default(); AIS.len()]; AIS.len()];
                loop {
                    if failed.load(Ordering::Relaxed) {
                        break;
                    }
                    let game_index = next_game.fetch_add(1, Ordering::Relaxed);
                    if game_index >= total_games {
                        break;
                    }

                    // Interleave the matchups so the early ETA includes both
                    // cheap and expensive search depths.
                    let matchup = game_index % (AIS.len() * AIS.len());
                    let white_index = matchup / AIS.len();
                    let black_index = matchup % AIS.len();
                    let mut game = EngineGame::new(
                        AIS[white_index].controller(),
                        AIS[black_index].controller(),
                        QuietUi,
                    );
                    let result = match game.play() {
                        Ok(GameOutcome::Checkmate {
                            winner: PieceColor::White,
                        }) => ResultForWhite::Win,
                        Ok(GameOutcome::Checkmate { .. }) => ResultForWhite::Loss,
                        Ok(GameOutcome::Stalemate | GameOutcome::FiftyMoveDraw) => {
                            ResultForWhite::Draw
                        }
                        Err(()) => {
                            failed.store(true, Ordering::Relaxed);
                            break;
                        }
                    };
                    results[white_index][black_index].add(result);
                    report_progress(
                        completed_games.fetch_add(1, Ordering::Relaxed) + 1,
                        total_games,
                        started,
                        &last_report,
                    );
                }
                results
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });

    if failed.load(Ordering::Relaxed) {
        eprintln!("An AI failed to provide a move in a playable position.");
        return;
    }

    let mut matchups = [[Record::default(); AIS.len()]; AIS.len()];
    for result in worker_results {
        let Ok(result) = result else {
            eprintln!("A benchmark worker panicked.");
            return;
        };
        for row in 0..AIS.len() {
            for column in 0..AIS.len() {
                matchups[row][column].merge(result[row][column]);
            }
        }
    }

    let totals = calculate_totals(&matchups);
    print_matchups(&matchups);
    print_totals(&totals);
}

fn report_progress(completed: usize, total: usize, started: Instant, last_report: &AtomicU64) {
    const REPORT_INTERVAL_SECS: u64 = 5;

    let elapsed = started.elapsed();
    let elapsed_secs = elapsed.as_secs();
    if completed != total {
        let previous = last_report.load(Ordering::Relaxed);
        if elapsed_secs < previous + REPORT_INTERVAL_SECS
            || last_report
                .compare_exchange(previous, elapsed_secs, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
        {
            return;
        }
    }

    let remaining = total - completed;
    let eta_secs = elapsed.as_secs_f64() * remaining as f64 / completed as f64;
    println!(
        "Progress: {completed}/{total} ({:.1}%) | elapsed {} | ETA {}",
        completed as f64 * 100.0 / total as f64,
        format_duration(elapsed.as_secs()),
        format_duration(eta_secs.round() as u64),
    );
    let _ = io::stdout().flush();
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3_600;
    let minutes = seconds % 3_600 / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn calculate_totals(matchups: &[[Record; AIS.len()]; AIS.len()]) -> [Record; AIS.len()] {
    let mut totals = [Record::default(); AIS.len()];
    for white in 0..AIS.len() {
        for black in 0..AIS.len() {
            let result = matchups[white][black];
            totals[white].merge(result);
            totals[black].merge(Record {
                wins: result.losses,
                losses: result.wins,
                draws: result.draws,
            });
        }
    }
    totals
}

fn print_matchups(matchups: &[[Record; AIS.len()]; AIS.len()]) {
    const WIDTH: usize = 15;

    println!("\nMatchups (row is White; each cell is wins-losses-draws):");
    print!("{:<WIDTH$}", "White / Black");
    for ai in AIS {
        print!(" {:>WIDTH$}", ai.name());
    }
    println!();

    for (row, ai) in AIS.into_iter().enumerate() {
        print!("{:<WIDTH$}", ai.name());
        for record in matchups[row] {
            let result = format!("{}-{}-{}", record.wins, record.losses, record.draws);
            print!(" {result:>WIDTH$}");
        }
        println!();
    }
}

fn print_totals(totals: &[Record; AIS.len()]) {
    println!("\nOverall standings (both colors):");
    println!(
        "{:<15} {:>8} {:>8} {:>8} {:>9}",
        "AI", "Wins", "Losses", "Draws", "Score"
    );
    for (ai, record) in AIS.into_iter().zip(totals) {
        let score_tenths = if record.games() == 0 {
            0
        } else {
            (record.wins * 1_000 + record.draws * 500) / record.games()
        };
        println!(
            "{:<15} {:>8} {:>8} {:>8} {:>8}.{}%",
            ai.name(),
            record.wins,
            record.losses,
            record.draws,
            score_tenths / 10,
            score_tenths % 10,
        );
    }
}
