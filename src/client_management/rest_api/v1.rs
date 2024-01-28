use crate::{DataProvider, Move};

use actix_web::{
    web::{Data, Json, Path},
    HttpRequest, Responder,
};
use serde::Deserialize;
use serde_json::to_string;
use std::sync::Mutex;
use uuid::Uuid;

pub async fn get_games<T: DataProvider>(
    _request: HttpRequest,
    games: Data<Mutex<T>>,
) -> impl Responder {
    let games = games.lock().unwrap();
    let games = games.get_games().unwrap();
    serde_json::to_string(&games).unwrap()
}

#[derive(Deserialize)]
pub struct GameSelector {
    game_id: Uuid,
}

pub async fn get_game<T: DataProvider>(
    path: Path<GameSelector>,
    _request: HttpRequest,
    games: Data<Mutex<T>>,
) -> impl Responder {
    let games = games.lock().unwrap();
    match games.get_game_data(path.game_id) {
        Ok(game_data) => to_string(&game_data).unwrap(),
        Err(err) => to_string(&err).unwrap(),
    }
}

pub async fn create_game<T: DataProvider>(
    _request: HttpRequest,
    games: Data<Mutex<T>>,
) -> impl Responder {
    let mut games = games.lock().unwrap();
    match games.create_game(None) {
        Ok(game_id) => to_string(&game_id).unwrap(),
        Err(err) => to_string(&err).unwrap(),
    }
}

pub async fn add_move<T: DataProvider>(
    _request: HttpRequest,
    path: Path<GameSelector>,
    games: Data<Mutex<T>>,
    body: Json<Move>,
) -> impl Responder {
    let mut games = games.lock().unwrap();
    let new_move = body.into_inner();
    match games.add_move(path.game_id, new_move) {
        Err(err) => to_string(&err).unwrap(),
        Ok(_) => to_string(&"ok").unwrap(),
    }
}
