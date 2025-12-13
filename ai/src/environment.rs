use tetris_core::tetris::{Game, GameConfig};

use crate::tetris::{Action, Snapshot, StateVec, step_inner, to_state};

/// A simple wrapper around a tetris game
pub struct Environment {
    pub game: Game,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            game: Game::new(GameConfig::default()),
        }
    }

    pub fn reset(&mut self) {
        self.game = Game::new(GameConfig::default());
    }

    pub fn state(&self) -> StateVec {
        to_state(&self.game)
    }

    pub fn step(&mut self, action: &Action) -> Snapshot {
        let (reward, done) = step_inner(&mut self.game, action);
        Snapshot {
            state: to_state(&self.game),
            reward,
            done,
        }
    }
}
