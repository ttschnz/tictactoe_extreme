use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DataProvider, GameData};

#[derive(Clone)]
pub struct CacheProviderArgs {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum CacheProviderErrorKind {
    LockError,
    KeyNotFound,
    GameExists,
}
impl ToString for CacheProviderErrorKind {
    fn to_string(&self) -> String {
        match self {
            CacheProviderErrorKind::GameExists => "the game allready exists",
            CacheProviderErrorKind::KeyNotFound => "the game does not exist",
            CacheProviderErrorKind::LockError => "could not aquire lock on hashmap",
        }
        .to_string()
    }
}

#[derive(Clone)]
pub struct CacheProvider {
    pub hash_map: Arc<Mutex<HashMap<Uuid, GameData>>>,
    pub channels: Arc<Mutex<HashMap<Uuid, Vec<tokio::sync::watch::Sender<GameData>>>>>,
}

impl Default for CacheProvider {
    fn default() -> Self {
        Self::new(CacheProviderArgs {}).unwrap()
    }
}

impl DataProvider for CacheProvider {
    type Args = CacheProviderArgs;
    type ErrorKind = CacheProviderErrorKind;
    fn add_move(&mut self, game_id: Uuid, new_move: crate::Move) -> Result<(), Self::ErrorKind> {
        let mut hash_map = self
            .hash_map
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?;
        hash_map
            .entry(game_id)
            .and_modify(|game_data| game_data.moves.push(new_move));

        let new_game_data = hash_map
            .get(&game_id)
            .ok_or(Self::ErrorKind::KeyNotFound)?
            .clone();

        if let Some(channels) = self
            .channels
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?
            .get(&game_id)
        {
            channels.iter().for_each(|channel| {
                channel.send(new_game_data.clone()).unwrap();
            })
        };

        Ok(())
    }
    fn create_game(&mut self, game_id: Option<Uuid>) -> Result<Uuid, Self::ErrorKind> {
        let game_id = game_id.unwrap_or_else(Uuid::new_v4);
        let mut hash_map = self
            .hash_map
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?;

        match hash_map.entry(game_id) {
            Entry::Occupied(_) => Err(Self::ErrorKind::GameExists),
            Entry::Vacant(entry) => {
                entry.insert(GameData::new_with_id(game_id));
                Ok(game_id)
            }
        }
    }
    fn game_exists(&mut self, game_id: Uuid) -> Result<bool, Self::ErrorKind> {
        let mut hash_map = self
            .hash_map
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?;

        Ok(matches!(hash_map.entry(game_id), Entry::Occupied(_)))
    }
    fn get_game_data(&self, game_id: Uuid) -> Result<crate::GameData, Self::ErrorKind> {
        let mut hash_map = self
            .hash_map
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?;
        match hash_map.entry(game_id) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(_) => Err(Self::ErrorKind::KeyNotFound),
        }
    }
    fn new(_args: Self::Args) -> Result<Self, Self::ErrorKind>
    where
        Self: Sized,
    {
        Ok(Self {
            hash_map: Arc::new(Mutex::new(HashMap::new())),
            channels: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    fn sync_board(&mut self, _game: &mut crate::Board) -> Result<(), Self::ErrorKind> {
        Ok(())
    }
    fn get_games(&self) -> Result<Vec<Uuid>, Self::ErrorKind> {
        let hash_map = self
            .hash_map
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?;
        Ok(hash_map.keys().cloned().collect())
    }

    fn subscribe_to_game(
        &mut self,
        game_id: Uuid,
    ) -> Result<tokio::sync::watch::Receiver<GameData>, Self::ErrorKind> {
        let (tx, rx) = tokio::sync::watch::channel(self.get_game_data(game_id)?);
        match self
            .channels
            .lock()
            .map_err(|_| Self::ErrorKind::LockError)?
            .entry(game_id)
        {
            Entry::Occupied(mut entry) => {
                let channels = entry.get_mut();
                channels.push(tx)
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![tx]);
            }
        };

        Ok(rx)
    }
}
