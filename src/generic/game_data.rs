use crate::Move;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TODO: is this abstraction layer necessary?
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameData {
    pub moves: Vec<Move>,
    pub game_id: Uuid,
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

impl GameData {
    pub fn new() -> Self {
        Self {
            moves: vec![],
            game_id: Uuid::new_v4(),
        }
    }
    pub fn new_with_id(id: Uuid) -> Self {
        Self {
            moves: vec![],
            game_id: id,
        }
    }

    pub fn add_move(&mut self, m: Move) {
        self.moves.push(m);
    }
}
