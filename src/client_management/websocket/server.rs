use crate::{websocket::StreamHandler, DataProvider, Server, ServerArgs};
use log::{debug, error};
use std::{future::Future, marker::PhantomData};
use tokio::{net::TcpListener, spawn};

#[derive(Debug)]
pub enum ErrorKind {
    InvalidAddress,
    ErrorListening(std::io::Error),
}

#[derive(Clone)]
pub struct WebSocketConfiguration {
    pub listen_port: u16,
    pub listen_addr: String,
}
impl WebSocketConfiguration {
    const DEFAULT_SOCKET_PORT: u16 = 8080;
    const DEFAULT_SOCKET_ADDR: &'static str = "127.0.0.1";
}
impl ServerArgs for WebSocketConfiguration {
    fn from_env() -> Self {
        let addr = std::env::var("WEBSOCKET_ADDR")
            .unwrap_or_else(|_| Self::DEFAULT_SOCKET_ADDR.to_string());
        let port = std::env::var("WEBSOCKET_PORT")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(Self::DEFAULT_SOCKET_PORT);

        WebSocketConfiguration {
            listen_addr: addr,
            listen_port: port,
        }
    }
}
pub struct WebSocketServer<T: DataProvider> {
    pub config: WebSocketConfiguration,
    phantom: PhantomData<T>,
}

impl<T: DataProvider + 'static> Server<T> for WebSocketServer<T> {
    type Args = WebSocketConfiguration;
    type ErrorKind = ErrorKind;
    fn new(config: Self::Args) -> Result<Self, Self::ErrorKind> {
        // validate addr
        config
            .listen_addr
            .parse::<std::net::IpAddr>()
            .map_err(|_| ErrorKind::InvalidAddress)?;

        Ok(Self {
            config,
            phantom: PhantomData,
        })
    }
    fn start(
        &mut self,
        data_provider: T,
    ) -> impl Future<Output = Result<(), Self::ErrorKind>> + Send {
        async move {
            let addr = format!("{}:{}", self.config.listen_addr, self.config.listen_port);
            debug!("Listening on {}", addr);
            let server = TcpListener::bind(addr)
                .await
                .map_err(|e| ErrorKind::ErrorListening(e))?;
            debug!("server started");
            loop {
                match server.accept().await {
                    Err(e) => {
                        error!("Error accepting connection: {:?}", e);
                    }
                    Ok((stream, _)) => {
                        debug!("new connection");
                        let data_provider = data_provider.clone();
                        spawn(async {
                            match StreamHandler::handle_stream(stream, data_provider).await {
                                Err(e) => error!("Error handling stream: {:?}", e),
                                _ => {}
                            }
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{websocket::stream_handler::IncommingMessage, CacheProvider, CacheProviderArgs};

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

        let config = WebSocketConfiguration::from_env();
        let data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();
        let mut server = WebSocketServer::new(config).unwrap();
        server.start(data_provider).await.unwrap();
    }

    #[tokio::test]
    async fn test_server() {
        env_logger::builder()
            .is_test(true)
            .try_init()
            .expect("Failed to init logger");
        let config = WebSocketConfiguration::from_env();
        let moving_config = config.clone();
        let data_provider = CacheProvider::new(CacheProviderArgs {}).unwrap();

        tokio::spawn(async move {
            let mut server = WebSocketServer::new(moving_config).unwrap();
            server.start(data_provider).await.unwrap();
        });

        // wait for server to start
        sleep(Duration::from_millis(100)).await;

        // connect to server
        match connect_async(format!(
            "ws://{}:{}/{}",
            config.listen_addr,
            config.listen_port,
            Uuid::new_v4()
        ))
        .await
        {
            Err(e) => {
                panic!("Error connecting to server: {:?}", e);
            }
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();
                write
                    .send(Message::Text("Hello".to_string()))
                    .await
                    .unwrap();

                let msg = read.next().await.unwrap();
                assert_eq!(msg.unwrap(), Message::Text("Hello".to_string()));
            }
        }
    }
}
