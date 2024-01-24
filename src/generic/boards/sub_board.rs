use ndarray::Array2;

use crate::{generic::boards::check_matrix, Coordinates, Field, GameState, Player};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubBoard {
    pub data: Array2<Field>,
    pub state: Field,
}

impl Default for SubBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl SubBoard {
    pub const SIZE: Coordinates = (3, 3);

    pub fn new() -> Self {
        Self {
            data: Array2::from_elem((Self::SIZE.0, Self::SIZE.1), Field::Vacant),
            state: Field::Vacant,
        }
    }

    pub fn get_state(&self, next_player: Player) -> GameState {
        check_matrix(&self.data, next_player)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Player;

    #[test]
    fn test_subboard() {
        let subboard = SubBoard::new();
        assert_eq!(subboard.data, Array2::from_elem((3, 3), Field::Vacant));
        assert_eq!(subboard.state, Field::Vacant);
    }

    #[test]
    fn test_full_subboard() {
        let mut subboard = SubBoard::new();
        subboard.data = Array2::from_elem((3, 3), Field::Occupied { player: Player::X });
        assert_eq!(
            subboard.get_state(Player::X),
            GameState::Won { winner: Player::X }
        );
    }
}
