use super::{Engine, Move, board::PieceColor};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    Checkmate { winner: PieceColor },
    Stalemate,
    FiftyMoveDraw,
}

pub trait GameInterface<Controller> {
    type Error;

    /// There is nothing stopping a controller from making an illegal move..
    /// but its soooooooo much extra compute to check..
    fn select_move(
        &mut self,
        controller: &mut Controller,
        engine: &Engine,
    ) -> Result<Move, Self::Error>;

    fn game_started(&mut self, _engine: &Engine) {}

    fn turn_started(&mut self, _engine: &Engine) {}

    fn move_made(&mut self, _engine: &Engine, _mov: Move) {}
}

pub struct Game<Controller, Interface> {
    engine: Engine,
    controllers: [Controller; 2],
    interface: Interface,
}

impl<Controller, Interface> Game<Controller, Interface> {
    pub fn new(white: Controller, black: Controller, interface: Interface) -> Self {
        Self {
            engine: Engine::default(),
            controllers: [white, black],
            interface,
        }
    }

    pub fn interface_mut(&mut self) -> &mut Interface {
        &mut self.interface
    }

    pub fn play(&mut self) -> Result<GameOutcome, Interface::Error>
    where
        Interface: GameInterface<Controller>,
    {
        self.interface.game_started(&self.engine);
        if let Some(outcome) = self.outcome() {
            return Ok(outcome);
        }

        loop {
            let mov = self.interface.select_move(
                &mut self.controllers[self.engine.turn() as usize],
                &self.engine,
            )?;

            self.engine.apply_move(mov);
            self.interface
                .move_made(&self.engine, mov);

            if let Some(outcome) = self.outcome() {
                return Ok(outcome);
            }
            self.interface.turn_started(&self.engine);
        }
    }

    fn outcome(&self) -> Option<GameOutcome> {
        let turn = self.engine.turn();
        if !self.engine.has_legal_move(turn) {
            if self.engine.is_in_check(turn) {
                Some(GameOutcome::Checkmate {
                    winner: turn.opposite(),
                })
            } else {
                Some(GameOutcome::Stalemate)
            }
        } else if self.engine.is_fifty_move_draw() {
            Some(GameOutcome::FiftyMoveDraw)
        } else {
            None
        }
    }
}
