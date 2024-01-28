use crate::DataProvider;
use actix_web::{
    web::{get, post, put, Data},
    App, HttpServer,
};
use std::sync::{Arc, Mutex};

mod v1;
use v1::{add_move, create_game, get_game, get_games};

/*
Endpoints:
* GET  /api/v1/games                 -> DataProvider::get_games
* GET  /api/v1/games/{game_id}       -> DataProvider::get_game_data(game_id)
* PUT  /api/v1/games                 -> DataProvider::create_game(None)
* POST /api/v1/games/{game_id}/moves -> DataProvider::add_move(game_id, body.move) // TODO: Add authentication

*/

/*
  TODO: Add authentication, not everyone should be able to make moves!
*
* An idea would be to send a token with the creation of the game which is the token for X,
* the first player to make a move is going to receive a token for O, these two tokens are
* going to be used to authenticate the moves.
* I currently see two ways of doing this:
*  - The tokens are randomized (uuids) and stored in the redis cache. This would keep the
*    implementation simple and the service scalable, allthough it would require more requests
*    to the redis cache and in the long term it would require more storage.
*  - The tokens are some kind of oauth tokens that are signed by the server. This would require
*    less requests to the redis cache and less storage, but it would require more implementation
*    plus we would have to find a way to store the private key for signing the tokens for the
*    service to remain scalable.
*
*/

pub struct ApiServer<T: DataProvider> {
    pub port: u16,
    pub host: String,
    pub data_provider: T,
}

impl<T: DataProvider + Default> Default for ApiServer<T> {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            data_provider: T::default(),
        }
    }
}

impl<T: DataProvider + 'static> ApiServer<T> {
    pub fn new(port: u16, host: String, data_provider: T) -> Self {
        Self {
            port,
            host,
            data_provider,
        }
    }

    fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub async fn start(&self) -> Result<(), std::io::Error> {
        let api = Arc::new(Mutex::new(self.data_provider.clone()));
        HttpServer::new(move || {
            let api = api.clone();
            App::new()
                .app_data(Data::new(api))
                // .route("/api/v1/games", web::get().to(api.get_games))
                .route("/api/v1/games", get().to(get_games::<T>))
                .route("/api/v1/games/{game_id}", get().to(get_game::<T>))
                .route("/api/v1/games", put().to(create_game::<T>))
                .route("/api/v1/games/{game_id}/moves", post().to(add_move::<T>))
        })
        .bind(self.get_address())
        .unwrap()
        .run()
        .await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{CacheProvider, CacheProviderArgs, GameData, Move, Player};
    use reqwest::{Client, StatusCode};
    use serial_test::serial;
    use uuid::Uuid;

    fn get_cache_api(existing_provider: Option<CacheProvider>) -> ApiServer<CacheProvider> {
        let random_port = rand::random::<u16>();
        ApiServer {
            port: random_port,
            data_provider: existing_provider.unwrap_or(CacheProvider::default()),
            ..Default::default()
        }
    }

    #[tokio::test]
    #[ignore = "this is a manual test"]
    async fn test_api_manual() {
        // std::env::set_var("RUST_LOG", "debug");
        // env_logger::builder()
        //     .is_test(true)
        //     .try_init()
        //     .expect("Failed to init logger");
        let api = get_cache_api(None);
        api.start().await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn get_games() {
        // std::env::set_var("RUST_LOG", "debug");
        // env_logger::builder()
        //     .is_test(true)
        //     .try_init()
        //     .expect("Failed to init logger");
        let mut data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();

        let mut game_uuids = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        for uuid in game_uuids.iter() {
            data_provider.create_game(Some(*uuid)).unwrap();
        }

        let api = get_cache_api(Some(data_provider));
        let addr = api.get_address();
        tokio::spawn(async move { api.start().await.unwrap() });

        let client = Client::new();
        let response = client
            .get(format!("http://{}/api/v1/games", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let mut remote_uuids =
            serde_json::from_str::<Vec<Uuid>>(&response.text().await.unwrap()).unwrap();

        assert_eq!(remote_uuids.len(), game_uuids.len());

        remote_uuids.sort();
        game_uuids.sort();

        assert_eq!(remote_uuids, game_uuids);
    }

    #[tokio::test]
    #[serial]
    async fn get_game() {
        // std::env::set_var("RUST_LOG", "debug");
        // env_logger::builder()
        //     .is_test(true)
        //     .try_init()
        //     .expect("Failed to init logger");
        let mut data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();

        let game_uuid = Uuid::new_v4();

        data_provider.create_game(Some(game_uuid)).unwrap();
        data_provider
            .add_move(game_uuid, Move::new((0, 0), Player::X))
            .unwrap();

        let data = data_provider.get_game_data(game_uuid).unwrap();

        let api = get_cache_api(Some(data_provider));
        let addr = api.get_address();
        tokio::spawn(async move { api.start().await.unwrap() });

        let client = Client::new();
        let response = client
            .get(format!("http://{}/api/v1/games/{}", addr, game_uuid))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let remote_data =
            serde_json::from_str::<GameData>(&response.text().await.unwrap()).unwrap();

        assert_eq!(remote_data, data);
    }

    #[tokio::test]
    #[serial]
    async fn add_move() {
        // std::env::set_var("RUST_LOG", "debug");
        // env_logger::builder()
        //     .is_test(true)
        //     .try_init()
        //     .expect("Failed to init logger");

        let mut data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();

        let game_uuid = Uuid::new_v4();

        data_provider.create_game(Some(game_uuid)).unwrap();

        let new_move = Move::new((0, 0), Player::X);

        let api = get_cache_api(Some(data_provider));
        let addr = api.get_address();
        tokio::spawn(async move { api.start().await.unwrap() });

        let client = Client::new();
        let response = client
            .post(format!("http://{}/api/v1/games/{}/moves", addr, game_uuid))
            .body(serde_json::to_string(&new_move).unwrap())
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get(format!("http://{}/api/v1/games/{}", addr, game_uuid))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let remote_data =
            serde_json::from_str::<GameData>(&response.text().await.unwrap()).unwrap();

        assert_eq!(remote_data.moves, vec![new_move]);
    }
}
