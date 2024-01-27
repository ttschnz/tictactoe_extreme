use itertools::Itertools;
use log::debug;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

use crate::{
    generic::boards::check_matrix, Coordinates, Field, GameData, GameState, Move, Player, SubBoard,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidMove {
    FieldOccupied,
    SubBoardNotActive,
    GameEnded,
    OutOfBounds,
    NotYourTurn,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Board {
    pub data: Array2<SubBoard>,
    pub moves: Vec<Move>,
    pub game_id: Uuid,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl From<GameData> for Board {
    fn from(game_data: GameData) -> Self {
        let mut board = Board::new_with_id(game_data.game_id);
        for m in game_data.moves {
            board
                .insert_move(m.coordinates, m.player)
                .expect("Invalid move in game data");
        }
        board
    }
}

impl Into<GameData> for Board {
    fn into(self) -> GameData {
        GameData {
            moves: self.moves,
            game_id: self.game_id,
        }
    }
}

impl Board {
    pub const SIZE: Coordinates = (3, 3);
    pub fn new() -> Self {
        Self {
            data: Array2::from_elem((Self::SIZE.0, Self::SIZE.1), SubBoard::new()),
            moves: Vec::new(),
            game_id: Uuid::new_v4(),
        }
    }

    pub fn new_with_id(id: Uuid) -> Self {
        Self {
            data: Array2::from_elem((Self::SIZE.0, Self::SIZE.1), SubBoard::new()),
            moves: Vec::new(),
            game_id: id,
        }
    }

    pub fn get_next_player(&self) -> Player {
        self.moves
            .last()
            .map(|last_move| last_move.player.other())
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
                    debug!("limiting subboard to {:?}", field_index);
                    Some(field_index)
                } else {
                    None
                }
            } else {
                None
            };

        let full_size = (
            0..Self::SIZE.0 * SubBoard::SIZE.0,
            0..Self::SIZE.1 * SubBoard::SIZE.1,
        );
        for (row, column) in full_size.0.cartesian_product(full_size.1) {
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

        debug!("allowed moves: {}", allowed_moves.len());
        allowed_moves
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

        check_matrix(&data, next_player)
    }

    /// Returns the subboard that the given move is in, and the coordinates of the move in that subboard
    /// Example: (5,1) -> ((1,0), (2,1))  1*3+2 = 5, 0*3+1 = 1
    fn get_subboard_for_move(
        &self,
        _coordinates: Coordinates,
    ) -> Option<(Coordinates, Coordinates)> {
        let (row, column) = _coordinates;

        let subboard_row = row / SubBoard::SIZE.0;
        let subboard_column = column / SubBoard::SIZE.1;

        let field_row = row - (subboard_row * SubBoard::SIZE.0);
        let field_column = column - (subboard_column * SubBoard::SIZE.1);

        Some(((subboard_row, subboard_column), (field_row, field_column)))
    }

    pub fn insert_move(
        &mut self,
        coordinates: Coordinates,
        player: Player,
    ) -> Result<(), InvalidMove> {
        let new_move = Move::new(coordinates, player);
        self.validate_move(new_move)?;
        self.moves.push(new_move);
        self.render_move(&new_move)?;
        Ok(())
    }

    pub fn validate_move(&self, new_move: Move) -> Result<(), InvalidMove> {
        // NotYourTurn
        if self.get_next_player() != new_move.player {
            return Err(InvalidMove::NotYourTurn);
        }
        // GameEnded
        if !self.get_state().is_in_progress() {
            return Err(InvalidMove::GameEnded);
        }
        // OutOfBounds
        if new_move.coordinates.0 >= Self::SIZE.0 * SubBoard::SIZE.0
            || new_move.coordinates.1 >= Self::SIZE.1 * SubBoard::SIZE.1
        {
            return Err(InvalidMove::OutOfBounds);
        }
        // FieldOccupied
        if let Some((_subboard_index, field_index)) =
            self.get_subboard_for_move(new_move.coordinates)
        {
            if self.data[_subboard_index].data[field_index] != Field::Vacant {
                return Err(InvalidMove::FieldOccupied);
            }
        }
        // SubBoardNotActive
        if !self.get_allowed_moves().contains(&new_move.coordinates) {
            return Err(InvalidMove::SubBoardNotActive);
        }
        Ok(())
    }

    pub fn render_move(&mut self, m: &Move) -> Result<(), InvalidMove> {
        let (subboard_index, field_index) = self
            .get_subboard_for_move(m.coordinates)
            .ok_or(InvalidMove::OutOfBounds)?;
        self.data[subboard_index].data[field_index] = Field::Occupied { player: m.player };
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use env_logger;

    // O   X |       | O
    //   X   |   O   |   O
    // X     |       |     O
    // ------+-------+------+
    //       | X X X |
    //       |   O   |
    //       |       |
    // ------+-------+------+
    //       |       |     X
    //       |       |   X
    //     O |       | X
    fn get_sample_game() -> Vec<Move> {
        vec![
            Move::new((1, 1), Player::X),
            Move::new((4, 4), Player::O),
            Move::new((3, 4), Player::X),
            Move::new((1, 4), Player::O),
            Move::new((3, 5), Player::X),
            Move::new((1, 7), Player::O),
            Move::new((3, 3), Player::X),
            Move::new((0, 0), Player::O),
            Move::new((0, 2), Player::X),
            Move::new((0, 6), Player::O),
            Move::new((2, 0), Player::X),
            Move::new((8, 2), Player::O),
            Move::new((6, 8), Player::X),
            Move::new((2, 8), Player::O),
            Move::new((7, 7), Player::X),
            Move::new((2, 5), Player::O),
            Move::new((8, 6), Player::X),
        ]
    }

    #[test]
    fn get_subboard_for_move() {
        let board = Board::new();
        assert_eq!(board.get_subboard_for_move((0, 0)), Some(((0, 0), (0, 0))));
        assert_eq!(board.get_subboard_for_move((1, 1)), Some(((0, 0), (1, 1))));
        assert_eq!(board.get_subboard_for_move((2, 2)), Some(((0, 0), (2, 2))));
        assert_eq!(board.get_subboard_for_move((3, 3)), Some(((1, 1), (0, 0))));
        assert_eq!(board.get_subboard_for_move((4, 4)), Some(((1, 1), (1, 1))));
        assert_eq!(board.get_subboard_for_move((5, 5)), Some(((1, 1), (2, 2))));
        assert_eq!(board.get_subboard_for_move((6, 6)), Some(((2, 2), (0, 0))));
        assert_eq!(board.get_subboard_for_move((7, 7)), Some(((2, 2), (1, 1))));
        assert_eq!(board.get_subboard_for_move((8, 8)), Some(((2, 2), (2, 2))));
    }

    #[test]
    fn get_state() {
        // enable logging
        // std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
        // the results associated with each move in SAMPLE_FULL_GAME
        let expected_results = vec![
            None,
            None,
            None,
            None,
            None,
            None,
            Some(vec![
                vec![Field::Vacant, Field::Vacant, Field::Vacant],
                vec![
                    Field::Vacant,
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                ],
                vec![Field::Vacant, Field::Vacant, Field::Vacant],
            ]),
            None,
            None,
            None,
            Some(vec![
                vec![
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                    Field::Vacant,
                ],
                vec![
                    Field::Vacant,
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                ],
                vec![Field::Vacant, Field::Vacant, Field::Vacant],
            ]),
            None,
            None,
            Some(vec![
                vec![
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                    Field::Occupied { player: Player::O },
                ],
                vec![
                    Field::Vacant,
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                ],
                vec![Field::Vacant, Field::Vacant, Field::Vacant],
            ]),
            None,
            None,
            Some(vec![
                vec![
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                    Field::Occupied { player: Player::O },
                ],
                vec![
                    Field::Vacant,
                    Field::Occupied { player: Player::X },
                    Field::Vacant,
                ],
                vec![
                    Field::Vacant,
                    Field::Vacant,
                    Field::Occupied { player: Player::X },
                ],
            ]),
        ];

        let expected_end_state = GameState::Won { winner: Player::X };

        let mut game_board = Board::new();
        let moves = get_sample_game();
        for (move_index, m) in moves.iter().enumerate() {
            let expected_abstraction = &expected_results[move_index];
            game_board.insert_move(m.coordinates, m.player).expect(
                format!("expected move {} to be valid, but it was not", move_index).as_str(),
            );
            if let Some(expected_abstraction) = expected_abstraction {
                assert_eq!(
                    game_board.get_abstracted_board(),
                    Array2::from_shape_vec(
                        (3, 3),
                        expected_abstraction
                            .iter()
                            .flatten()
                            .cloned()
                            .collect::<Vec<Field>>()
                    )
                    .unwrap(),
                    "Test failed: got abstraction {:?} instead of {:?}",
                    game_board.get_abstracted_board(),
                    expected_abstraction
                );
            }
        }
        assert_eq!(
            game_board.get_state(),
            expected_end_state,
            "Test failed: got state {} instead of {}",
            game_board.get_state(),
            expected_end_state
        );
    }

    #[test]
    fn validate_move() {
        // things to test:
        // InvalidMove::FieldOccupied;
        // InvalidMove::GameEnded;
        // InvalidMove::NotYourTurn;
        // InvalidMove::OutOfBounds;
        // InvalidMove::SubBoardNotActive;

        let tests = vec![
            vec![
                (Move::new((0, 0), Player::X), None),
                (
                    Move::new((0, 0), Player::O),
                    Some(InvalidMove::FieldOccupied),
                ),
            ],
            // moves from SAMPLE_FULL_GAME, and a new move that should give GameEnded
            get_sample_game()
                .iter()
                .map(|m| (m.clone(), None))
                .chain(vec![(
                    Move::new((7, 0), Player::O),
                    Some(InvalidMove::GameEnded),
                )])
                .collect(),
            vec![
                (Move::new((0, 0), Player::O), Some(InvalidMove::NotYourTurn)),
                (Move::new((0, 0), Player::O), Some(InvalidMove::NotYourTurn)),
                (Move::new((0, 0), Player::O), Some(InvalidMove::NotYourTurn)),
                (Move::new((0, 0), Player::O), Some(InvalidMove::NotYourTurn)),
                (Move::new((4, 4), Player::X), None), // after an invalid move, nothing should be changed in the board
                (Move::new((3, 3), Player::X), Some(InvalidMove::NotYourTurn)),
                (Move::new((3, 3), Player::X), Some(InvalidMove::NotYourTurn)),
                (Move::new((3, 3), Player::O), None),
            ],
            vec![
                (Move::new((9, 9), Player::X), Some(InvalidMove::OutOfBounds)),
                (Move::new((0, 0), Player::X), None),
                (Move::new((0, 9), Player::O), Some(InvalidMove::OutOfBounds)),
                (Move::new((0, 1), Player::O), None),
                (Move::new((0, 9), Player::X), Some(InvalidMove::OutOfBounds)),
            ],
            vec![
                (Move::new((0, 0), Player::X), None),
                (
                    Move::new((4, 4), Player::O),
                    Some(InvalidMove::SubBoardNotActive),
                ),
            ],
        ];

        for (test_index, board) in tests.iter().enumerate() {
            let mut game_board = Board::new();
            for (move_index, (new_move, expected_error)) in board.iter().enumerate() {
                let result = game_board.insert_move(new_move.coordinates, new_move.player);
                if let Some(expected_error) = expected_error {
                    assert_eq!(
                        result,
                        Err(*expected_error),
                        "Test {} failed: got {:?} instead of {:?} (move {})",
                        test_index,
                        result,
                        expected_error,
                        move_index
                    );
                } else {
                    assert_eq!(
                        result,
                        Ok(()),
                        "Test {} failed: got {:?} instead of Ok(()) (move {})",
                        test_index,
                        result,
                        move_index
                    );
                }
            }
        }
    }

    // from and into game data
    #[test]
    fn from_game_data() {
        let mut board = Board::new();

        for new_move in get_sample_game() {
            board
                .insert_move(new_move.coordinates, new_move.player)
                .expect(format!("expected to be able to add move {:?}", new_move).as_str());
        }
        assert_eq!(board.get_state(), GameState::Won { winner: Player::X });

        let game_data: GameData = board.into();

        let board = Board::from(game_data);
        assert_eq!(board.moves, get_sample_game());
        assert_eq!(board.get_state(), GameState::Won { winner: Player::X });
    }
}
