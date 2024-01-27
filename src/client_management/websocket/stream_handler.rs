use crate::{Board, Coordinates, DataProvider, Move, Player};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use uuid::Uuid;

use log::{debug, error, warn};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Error as WebSocketError,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OutgoingMessage {
    AskForMove { player: Player, game_state: Board },
    Welcome { player: Player, game_uuid: Uuid },
    GameState { player: Player, game_state: Board },
    Error { error_message: Error },
    Pong {},
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum IncommingMessage {
    MakeMove {
        player: Player,
        coordinates: Coordinates,
    },
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

pub enum ClientType {
    Player { role: Player },
    Spectator,
}

pub struct StreamHandler<T: DataProvider> {
    pub client_type: ClientType,
    pub stream: WebSocketStream<TcpStream>,
    pub connected_game: Uuid,
    pub data_provider: T,
}

impl<T: DataProvider> StreamHandler<T> {
    fn handle_message(&mut self, msg: String) -> Option<String> {
        match serde_json::from_str::<IncommingMessage>(&msg) {
            Ok(IncommingMessage::MakeMove {
                player,
                coordinates,
            }) => match self
                .data_provider
                .add_move(self.connected_game, Move::new(coordinates, player))
            {
                Err(err_reason) => Some(OutgoingMessage::Error {
                    error_message: Error::ErrorMakingMove(err_reason.to_string()),
                }),
                Ok(()) => None,
            },

            Ok(IncommingMessage::Ping {}) => Some(OutgoingMessage::Pong {}),
            Err(_) => Some(OutgoingMessage::Error {
                error_message: Error::CouldNotSerialize(msg),
            }),
        }
        .map(|response| serde_json::to_string(&response).unwrap())
    }

    pub async fn handle_stream(stream: TcpStream, data_provider: T) -> Result<(), Error> {
        let client = Self::accept_connection(stream, data_provider).await;
        match client {
            Err(e) => {
                error!("Error accepting connection: {:?}", e);
                Err(e)
            }
            Ok(mut client) => {
                debug!("Client accepted");
                client.handle_messages().await;
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
        // path is made of two arguments: /<game_uuid>/(<player>|"watch")
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

        let client_type = match path.nth(0) {
            None | Some("watch") => ClientType::Spectator,
            Some("X") => ClientType::Player { role: Player::X },
            Some("O") => ClientType::Player { role: Player::O },
            Some(role) => return Err(Error::InvalidRole(role.to_string())),
        };

        Ok(Self {
            client_type,
            stream,
            connected_game: game_id,
            data_provider,
        })
    }

    async fn handle_messages(&mut self) {
        debug!("Handling messages");
        while let Some(msg) = self.stream.next().await {
            match msg {
                Err(WebSocketError::AlreadyClosed) | Err(WebSocketError::ConnectionClosed) => {
                    error!("Connection closed");
                    break;
                }
                Err(WebSocketError::AttackAttempt)
                | Err(WebSocketError::Http(_))
                | Err(WebSocketError::HttpFormat(_))
                | Err(WebSocketError::Io(_))
                | Err(WebSocketError::Tls(_))
                | Err(WebSocketError::Url(_)) => {
                    error!("Error during the websocket handshake occurred: {:?}", msg);
                }

                Err(WebSocketError::Capacity(_))
                | Err(WebSocketError::Protocol(_))
                | Err(WebSocketError::Utf8)
                | Err(WebSocketError::WriteBufferFull(_)) => {
                    error!("Errorful websocket message: {:?}", msg);
                }

                Ok(msg) => {
                    debug!("Server received a message from client: {:?}", msg);
                    if let Ok(msg) = msg.into_text() {
                        if let Some(response) = self.handle_message(msg) {
                            if let Err(e) = self.stream.send(Message::Text(response)).await {
                                warn!("Failed to send response {}", e)
                            }
                        }
                    }
                }
            }
        }
    }
}
