mod factory;
mod providers;

pub use factory::DataProviderFactory;
pub use providers::*;

use crate::{GameData, Move};
use core::fmt::Debug;
use uuid::Uuid;

/// DataProvider is a trait that defines the interface for a data provider.
/// The operator of this trait doesn't care where the data is stored, it may
/// be for example in redis, in a file or in memory.
pub trait DataProvider: Sized {
    type Args: Clone;
    type ErrorKind: Debug + Clone + PartialEq + Eq;
    /// returns the game data for a given game id.
    /// This means that it will have  to fetch the data from its source,
    /// serialize it if needed and return it.
    fn get_game_data(&self, game_id: Uuid) -> Result<GameData, Self::ErrorKind>;

    /// adds a move to the game for a given game id.
    fn add_move(&mut self, game_id: Uuid, new_move: Move) -> Result<(), Self::ErrorKind>;

    /// creates a new game and returns the game id.
    fn create_game(&mut self) -> Result<Uuid, Self::ErrorKind>;

    fn new(args: Self::Args) -> Result<Self, Self::ErrorKind>;
}
