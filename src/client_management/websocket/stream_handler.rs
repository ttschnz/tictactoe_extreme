use crate::{Board, DataProvider, Player};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::WatchStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use uuid::Uuid;

use log::{debug, error};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::{Request, Response},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OutgoingMessage {
    Error { error_message: Error },
    Welcome { game_uuid: Uuid },
    GameState { game_state: Board },
    Pong {},
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum IncommingMessage {
    Ping {},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    InvalidUuid(String),
    GameNotFound,
    InvalidRole(String),
    HandShakeError(String),
    CouldNotSerialize(String),
    ErrorMakingMove(String),
}

pub struct StreamHandler<T: DataProvider> {
    pub stream: WebSocketStream<TcpStream>,
    pub connected_game: Uuid,
    pub data_provider: T,
}

impl<T: DataProvider> StreamHandler<T> {
    pub async fn handle_stream(stream: TcpStream, mut data_provider: T) -> Result<(), Error> {
        let client = Self::accept_connection(stream, data_provider.clone()).await;
        match client {
            Err(e) => {
                error!("Error accepting connection: {:?}", e);
                Err(e)
            }
            Ok(client) => {
                debug!("Client accepted");

                let mut rx = WatchStream::new(
                    data_provider
                        .subscribe_to_game(client.connected_game)
                        .unwrap(),
                );

                let (mut ws_sender, _) = client.stream.split();

                while let Some(game_data_update) = rx.next().await {
                    ws_sender
                        .send(Message::Text(
                            serde_json::to_string(&OutgoingMessage::GameState {
                                game_state: Board::from(game_data_update),
                            })
                            .unwrap(),
                        ))
                        .await
                        .unwrap();
                }
                Ok(())
            }
        }
    }

    async fn accept_connection(stream: TcpStream, data_provider: T) -> Result<Self, Error> {
        let request_path = Arc::new(Mutex::new(String::new()));

        let ws_stream = accept_hdr_async(stream, |req: &Request, response: Response| {
            *(request_path.lock().unwrap()) = req.uri().path().to_string();
            Ok(response)
        })
        .await
        .map_err(|ws_err| Error::HandShakeError(ws_err.to_string()))?;

        let path = request_path.lock().unwrap().deref().clone();
        debug!("request path: {:?}", path);
        StreamHandler::from_path(path, ws_stream, data_provider).await
    }

    async fn from_path(
        path: String,
        mut stream: WebSocketStream<TcpStream>,
        mut data_provider: T,
    ) -> Result<Self, Error> {
        // path is made of the game_uuid: /<game_uuid>
        // parse path
        let mut path = path.split('/');
        let game_uuid = path
            .nth(1)
            .ok_or_else(|| Error::InvalidUuid("No game uuid provided".to_string()))?;

        let game_id = Uuid::parse_str(game_uuid)
            .map_err(|_| Error::InvalidUuid(format!("Invalid game uuid: {}", game_uuid)))?;

        // check if uuid exists
        if !data_provider.game_exists(game_id).unwrap_or(false) {
            stream.close(None).await.unwrap();
            return Err(Error::GameNotFound);
        }

        Ok(Self {
            stream,
            connected_game: game_id,
            data_provider,
        })
    }
}
