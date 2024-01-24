use crate::{Field, GameState, Player};
use ndarray::{s, Array1, Array2, ArrayBase, ViewRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct WinnerRegisterer {
    winner: Option<Player>,
}
impl WinnerRegisterer {
    pub fn register(&mut self, player: Option<Player>) {
        if self.winner.is_none() && player.is_some() {
            self.winner = player;
        }
    }
    pub fn get_winner(&self) -> Option<Player> {
        self.winner
    }
}

/// Checks if a matrix of fields contains a winner
///
/// if one player has all fields in a row, column or diagonal, they win.
/// the first player that is found to have won is returned.
///
/// Usage:
/// ```
/// use ndarray::array;
/// use tictactoe_v2::{Field, Player, GameState, check_matrix};
///
/// // O X X
/// // O O O <-- O wins
/// // X O X
/// let matrix = array![
///    [Field::Occupied{ player: Player::O}, Field::Occupied{ player: Player::X}, Field::Occupied{ player: Player::X}],
///    [Field::Occupied{ player: Player::O}, Field::Occupied{ player: Player::O}, Field::Occupied{ player: Player::O}],
///    [Field::Occupied{ player: Player::X}, Field::Occupied{ player: Player::O}, Field::Occupied{ player: Player::X}],
/// ];
///
/// assert_eq!(check_matrix(&matrix, Player::X), GameState::Won{ winner: Player::O});
///
/// ```
///
pub fn check_matrix(matrix: &Array2<Field>, next_player: Player) -> GameState {
    let mut winner_registerer = WinnerRegisterer::default();

    // check diagonal
    winner_registerer.register(get_winner_in_row(matrix.diag()));

    // check anti-diagonal
    winner_registerer.register(get_winner_in_row(matrix.slice(s![..;-1, ..]).diag()));

    // check rows
    for row in matrix.rows() {
        winner_registerer.register(get_winner_in_row(row));
    }

    // check columns
    for column in matrix.columns() {
        winner_registerer.register(get_winner_in_row(column));
    }

    if let Some(winner) = winner_registerer.get_winner() {
        GameState::Won { winner }
    } else if matrix.iter().all(|field| !matches!(field, Field::Vacant)) {
        return GameState::Draw;
    } else {
        return GameState::InProgress { next_player };
    }
}

/// Checks if a list of fields contains a winner
///
/// This function is used by check_matrix to check rows, columns and diagonals.
fn get_winner_in_row(
    list: ArrayBase<ViewRepr<&Field>, ndarray::prelude::Dim<[usize; 1]>>,
) -> Option<Player> {
    let potential_winner = list.get(0)?;

    if Array1::from_elem(list.len(), *potential_winner) == list {
        match potential_winner {
            Field::Occupied { player } => Some(*player),
            _ => None,
        }
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Field;
    use ndarray::array;
    #[test]
    fn matrix_checker() {
        // O X X
        // O O O <-- O wins
        // X O X
        let matrix = array![
            [
                Field::Occupied { player: Player::O },
                Field::Occupied { player: Player::X },
                Field::Occupied { player: Player::X }
            ],
            [
                Field::Occupied { player: Player::O },
                Field::Occupied { player: Player::O },
                Field::Occupied { player: Player::O }
            ],
            [
                Field::Occupied { player: Player::X },
                Field::Occupied { player: Player::O },
                Field::Occupied { player: Player::X }
            ],
        ];
        assert_eq!(
            check_matrix(&matrix, Player::X),
            GameState::Won { winner: Player::O }
        );
    }
}
