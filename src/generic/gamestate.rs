use std::fmt::Display;

use crate::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Won { winner: Player },
    Draw,
    InProgress { next_player: Player },
}

impl Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameState::Won { winner } => write!(f, "Won by {}", winner),
            GameState::Draw => write!(f, "Draw"),
            GameState::InProgress { next_player } => write!(f, "Next player: {}", next_player),
        }
    }
}

impl GameState {
    pub fn is_won(&self) -> bool {
        matches!(self, GameState::Won { .. })
    }
    pub fn is_draw(&self) -> bool {
        matches!(self, GameState::Draw)
    }
    pub fn is_in_progress(&self) -> bool {
        matches!(self, GameState::InProgress { .. })
    }
}
