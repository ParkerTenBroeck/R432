use self::game::Game;

mod game;
#[allow(dead_code)]
pub mod mmio;
mod panic;
#[allow(dead_code)]
mod printer;
mod start;
mod time;

pub fn run() {
    let game = Game::new();
    game.begin_play();
}
