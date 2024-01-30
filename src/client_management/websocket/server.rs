use crate::{websocket::StreamHandler, DataProvider, Server};
use log::{debug, error};
use tokio::{net::TcpListener, spawn};

#[derive(Debug)]
pub enum ErrorKind {
    InvalidAddress,
    ErrorListening(std::io::Error),
}

#[derive(Clone)]
pub struct WebSocketServer<T: DataProvider> {
    pub port: u16,
    pub host: String,
    pub data_provider: T,
}

impl<T: DataProvider + Default + 'static> Server<T> for WebSocketServer<T> {
    type ErrorKind = ErrorKind;
    fn from_env(data_provider: T) -> Self {
        let host =
            std::env::var("WEBSOCKET_HOST").unwrap_or_else(|_| Self::DEFAULT_HOST.to_string());
        let port = std::env::var("WEBSOCKET_PORT")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(Self::DEFAULT_PORT);

        WebSocketServer {
            host,
            port,
            data_provider,
        }
    }
    fn new(host: String, port: u16, data_provider: T) -> Self {
        Self {
            host,
            port,
            data_provider,
        }
    }

    fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn default() -> Self {
        Self {
            host: Self::DEFAULT_HOST.to_string(),
            port: Self::DEFAULT_PORT,
            data_provider: T::default(),
        }
    }

    fn with_data_provider(data_provider: T) -> Self {
        Self::new(
            Self::DEFAULT_HOST.to_string(),
            Self::DEFAULT_PORT,
            data_provider,
        )
    }

    async fn start(&mut self) -> Result<(), Self::ErrorKind> {
        let addr = self.get_address();
        debug!("Listening on {}", addr);

        let server = TcpListener::bind(addr)
            .await
            .map_err(ErrorKind::ErrorListening)?;

        debug!("server started");

        loop {
            match server.accept().await {
                Err(e) => {
                    error!("Error accepting connection: {:?}", e);
                }
                Ok((stream, _)) => {
                    debug!("new connection");
                    let data_provider = self.data_provider.clone();
                    spawn(async {
                        if let Err(e) = StreamHandler::handle_stream(stream, data_provider).await {
                            error!("Error handling stream: {:?}", e)
                        }
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        websocket::stream_handler::{IncommingMessage, OutgoingMessage},
        CacheProvider,
    };

    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use uuid::Uuid;

    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

    use tokio::time::sleep;
    // use tokio_tungstenite::tungstenite::client;

    #[tokio::test]
    #[ignore = "this is a manual test"]
    async fn test_server_manual() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::builder()
            .is_test(true)
            .try_init()
            .expect("Failed to init logger");

        debug!("sample uuid {}", Uuid::new_v4().to_string());
        debug!(
            "sample ping {}",
            serde_json::to_string(&IncommingMessage::Ping {}).unwrap()
        );

        let mut server = WebSocketServer::<CacheProvider>::default();
        server.start().await.unwrap();
    }

    #[tokio::test]
    async fn test_server() {
        // env_logger::builder()
        //     .is_test(true)
        //     .try_init()
        //     .expect("Failed to init logger");
        let game_id = Uuid::new_v4();

        let mut server = WebSocketServer::from_env(CacheProvider::default());
        let server_address = server.get_address();
        let mut data_provider = CacheProvider::default();

        data_provider.create_game(Some(game_id)).unwrap();
        tokio::spawn(async move {
            server.start().await.unwrap();
        });

        // wait for server to start
        sleep(Duration::from_millis(100)).await;

        // connect to server
        match connect_async(format!("ws://{}/{}", server_address, game_id)).await {
            Err(e) => {
                panic!("Error connecting to server: {:?}", e);
            }
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();
                write
                    .send(Message::Text(
                        serde_json::to_string(&IncommingMessage::Ping {}).unwrap(),
                    ))
                    .await
                    .unwrap();

                let msg = read.next().await.unwrap().unwrap();
                serde_json::from_str::<OutgoingMessage>(&msg.to_string()).unwrap();
            }
        }
    }
}
