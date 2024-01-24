use crate::Move;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameData {
    pub moves: Vec<Move>,
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

impl GameData {
    pub fn new() -> Self {
        Self { moves: vec![] }
    }

    pub fn add_move(&mut self, m: Move) {
        self.moves.push(m);
    }
}
