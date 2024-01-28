use crate::DataProvider;
use std::sync::Mutex;

mod v1;
use v1::{add_move, create_game, get_game, get_games};

//
// Endpoints:
// - GET /api/v1/games -> DataProvider::get_games
// - GET /api/v1/games/{game_id} -> DataProvider::get_game_data(game_id)
// - PUT /api/v1/games -> DataProvider::create_game(None)
// - POST /api/v1/games/{game_id}/moves -> DataProvider::add_move(game_id, body.move)
//

use actix_web::{
    web::{get, post, put, Data},
    App, HttpServer,
};

pub async fn start_rest_api<T: DataProvider + 'static>(
    data_provider: T,
) -> Result<(), std::io::Error> {
    HttpServer::new(move || {
        let api = Mutex::new(data_provider.clone());

        App::new()
            .app_data(Data::new(api))
            // .route("/api/v1/games", web::get().to(api.get_games))
            .route("/api/v1/games", get().to(get_games::<T>))
            .route("/api/v1/games/{game_id}", get().to(get_game::<T>))
            .route("/api/v1/games", put().to(create_game::<T>))
            .route("/api/v1/games/{game_id}/moves", post().to(add_move::<T>))
    })
    .bind("127.0.0.1:8080")
    .unwrap()
    .run()
    .await
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{CacheProvider, CacheProviderArgs, GameData, Move, Player};
    use uuid::Uuid;

    use reqwest::{Client, StatusCode};

    #[tokio::test]
    #[ignore = "this is a manual test"]
    async fn test_api_manual() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::builder()
            .is_test(true)
            .try_init()
            .expect("Failed to init logger");
        let data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();
        start_rest_api(data_provider).await.unwrap();
    }

    #[tokio::test]
    async fn get_games() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::builder()
            .is_test(true)
            .try_init()
            .expect("Failed to init logger");
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

        tokio::spawn(async { start_rest_api(data_provider).await.unwrap() });

        let client = Client::new();
        let response = client
            .get("http://127.0.0.1:8080/api/v1/games")
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
    async fn get_game() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::builder()
            .is_test(true)
            .try_init()
            .expect("Failed to init logger");
        let mut data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();

        let game_uuid = Uuid::new_v4();

        data_provider.create_game(Some(game_uuid)).unwrap();
        data_provider
            .add_move(game_uuid, Move::new((0, 0), Player::X))
            .unwrap();

        let data = data_provider.get_game_data(game_uuid).unwrap();

        tokio::spawn(async { start_rest_api(data_provider).await.unwrap() });

        let client = Client::new();
        let response = client
            .get(format!("http://127.0.0.1:8080/api/v1/games/{}", game_uuid))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let remote_data =
            serde_json::from_str::<GameData>(&response.text().await.unwrap()).unwrap();

        assert_eq!(remote_data, data);
    }

    #[tokio::test]
    async fn add_move() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::builder()
            .is_test(true)
            .try_init()
            .expect("Failed to init logger");

        let mut data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();

        let game_uuid = Uuid::new_v4();

        data_provider.create_game(Some(game_uuid)).unwrap();

        let new_move = Move::new((0, 0), Player::X);

        tokio::spawn(async { start_rest_api(data_provider).await.unwrap() });

        let client = Client::new();
        let response = client
            .post(format!(
                "http://127.0.0.1:8080/api/v1/games/{}/moves",
                game_uuid
            ))
            .body(serde_json::to_string(&new_move).unwrap())
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get(format!("http://127.0.0.1:8080/api/v1/games/{}", game_uuid))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let remote_data =
            serde_json::from_str::<GameData>(&response.text().await.unwrap()).unwrap();

        assert_eq!(remote_data.moves, vec![new_move]);
    }
}
