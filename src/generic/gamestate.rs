use crate::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Won { winner: Player },
    Draw,
    InProgress { next_player: Player },
}
