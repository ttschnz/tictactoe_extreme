use ndarray::Array2;

// enum ErrorKind {

// }

use crate::{Coordinates, Field, GameState, Move, Player};

pub struct Board {
    pub data: Array2<SubBoard>,
    pub moves: Vec<Move>,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    pub const SIZE: Coordinates = (3, 3);
    pub fn new() -> Self {
        Self {
            data: Array2::from_elem(
                (Self::SIZE.0, Self::SIZE.1),
                SubBoard::new(),
            ),
            moves: Vec::new(),
        }
    }

    pub fn get_next_player(&self) -> Player {
        self.moves
            .last()
            .map(|last_move| last_move.player)
            .unwrap_or(Player::X)
    }

    pub fn get_allowed_moves(&self) -> Vec<Coordinates> {
        let current_states = self.get_abstracted_board();

        let mut allowed_moves = Vec::new();

        let limiting_subboard: Option<(usize, usize)> =
            if let Some((_subboard_index, field_index)) = self
                .moves
                .last()
                .and_then(|last_move| self.get_subboard_for_move(last_move.coordinates))
            {
                // the index of the last move in the field in the subboard is the index of the subboard
                // where the next move must be made. If this subboard is not vacand, the next move can
                // be made anywhere
                if current_states[field_index] == Field::Vacant {
                    Some(field_index)
                } else {
                    None
                }
            } else {
                None
            };

        for ((row, column), _) in self.data.indexed_iter() {
            let coordinates = (row, column);
            let (subboard_index, field_index) = self
                .get_subboard_for_move(coordinates)
                .expect("This should never happen");

            // if the subboard is not the limiting subboard, skip it
            if Some(subboard_index) != limiting_subboard && limiting_subboard.is_some() {
                continue;
            }

            if current_states[subboard_index] == Field::Vacant
                && self.data[subboard_index].data[field_index] == Field::Vacant
            {
                allowed_moves.push(coordinates);
            }
        }

        todo!()
    }

    fn get_abstracted_board(&self) -> Array2<Field> {
        let next_player = self.get_next_player();
        let shape = self.data.shape();
        let mut data = Array2::from_elem((shape[0], shape[1]), Field::Vacant);
        for (row_index, row) in self.data.rows().into_iter().enumerate() {
            for (column_index, sub_board) in row.iter().enumerate() {
                data[(row_index, column_index)] = match sub_board.get_state(next_player) {
                    GameState::InProgress { .. } => Field::Vacant,
                    GameState::Draw => Field::Disabled,
                    GameState::Won { winner } => Field::Occupied { player: winner },
                };
            }
        }
        data
    }

    pub fn get_state(&self) -> GameState {
        let next_player = self.get_next_player();
        let data = self.get_abstracted_board();
        matrix_checker::check_matrix(&data, next_player)
    }

    /// Returns the subboard that the given move is in, and the coordinates of the move in that subboard
    /// Example: (5,1) -> ((1,0), (2,1))  1*3+2 = 5, 0*3+1 = 1
    fn get_subboard_for_move(
        &self,
        _coordinates: Coordinates,
    ) -> Option<(Coordinates, Coordinates)> {
        todo!()
    }
}

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
            data: Array2::from_elem(
                (Self::SIZE.0, Self::SIZE.1),
                Field::Vacant,
            ),
            state: Field::Vacant,
        }
    }

    pub fn get_state(&self, next_player: Player) -> GameState {
        matrix_checker::check_matrix(&self.data, next_player)
    }
}

mod matrix_checker {
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
    /// use tictactoe::game::{Field, Player, utils::check_matrix, GameState};
    ///
    /// // O X X
    /// // O O O <-- O wins
    /// // X O X
    /// let matrix = array![
    ///    [Field::Occupied(Player::O), Field::Occupied(Player::X), Field::Occupied(Player::X)],
    ///    [Field::Occupied(Player::O), Field::Occupied(Player::O), Field::Occupied(Player::O)],
    ///    [Field::Occupied(Player::X), Field::Occupied(Player::O), Field::Occupied(Player::X)],
    /// ];
    ///
    /// assert_eq!(check_matrix(&matrix, Player::X), GameState::Won(Player::O));
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
}
