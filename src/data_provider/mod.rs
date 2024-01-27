mod factory;
mod providers;

pub use factory::DataProviderFactory;
pub use providers::*;

use crate::{Board, GameData, Move};
use core::fmt::Debug;
use uuid::Uuid;

/// DataProvider is a trait that defines the interface for a data provider.
/// The operator of this trait doesn't care where the data is stored, it may
/// be for example in redis, in a file or in memory.
pub trait DataProvider: Send + Clone {
    type Args: Clone;
    type ErrorKind: Debug + Clone + PartialEq + Eq + ToString;
    /// returns the game data for a given game id.
    /// This means that it will have  to fetch the data from its source,
    /// serialize it if needed and return it.
    fn get_game_data(&self, game_id: Uuid) -> Result<GameData, Self::ErrorKind>;

    /// adds a move to the game for a given game id.
    fn add_move(&mut self, game_id: Uuid, new_move: Move) -> Result<(), Self::ErrorKind>;

    /// creates a new game and returns the game id.
    fn create_game(&mut self, uuid: Option<Uuid>) -> Result<Uuid, Self::ErrorKind>;

    fn new(args: Self::Args) -> Result<Self, Self::ErrorKind>
    where
        Self: Sized;

    fn subscribe_to_game(
        &mut self,
        game_id: Uuid,
    ) -> Result<tokio::sync::watch::Receiver<GameData>, Self::ErrorKind>;

    /// checks if a game exists for a given game id.
    fn game_exists(&mut self, game_id: Uuid) -> Result<bool, Self::ErrorKind>;

    /// syncs the board with the data provider.
    /// This means check if there are any remote moves that are not in the board
    /// and add them to the board, and check if there are any local moves that
    /// are not in the data provider and add them to the data provider.
    ///
    /// If there are any conflicts, the remote moves should be prioritized.
    ///
    // TODO: How do we verify that the remote moves are valid?
    fn sync_board(&mut self, game: &mut Board) -> Result<(), Self::ErrorKind>;
}
