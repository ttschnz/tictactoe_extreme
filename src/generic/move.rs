use crate::Player;
use serde::{Deserialize, Serialize};

pub type Coordinates = (usize, usize);

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub coordinates: Coordinates,
    pub player: Player,
}

impl Move {
    pub fn new(coordinates: Coordinates, player: Player) -> Self {
        Self {
            coordinates,
            player,
        }
    }
}
