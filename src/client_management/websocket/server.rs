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
    pub mod redis_stack {
        use testcontainers::{core::WaitFor, Image};
        // docker image: redis-stack-server

        const NAME: &str = "redis/redis-stack-server";
        const TAG: &str = "latest";

        #[derive(Debug, Default)]
        pub struct Redis;

        impl Image for Redis {
            type Args = ();

            fn name(&self) -> String {
                NAME.to_owned()
            }

            fn tag(&self) -> String {
                TAG.to_owned()
            }

            fn ready_conditions(&self) -> Vec<WaitFor> {
                vec![WaitFor::message_on_stdout("Ready to accept connections")]
            }
        }
    }

    use super::*;
    use crate::{
        websocket::stream_handler::{IncommingMessage, OutgoingMessage},
        CacheProvider, Move, Player, RedisProvider, RedisProviderArgs,
    };
    use futures_util::StreamExt;

    use redis_stack::Redis;
    use std::time::Duration;
    use testcontainers::clients::Cli as DockerCli;
    use tokio::time::sleep;
    use tokio::time::timeout;
    use tokio_tungstenite::connect_async;
    use uuid::Uuid;

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
    async fn test_server_with_cache() {
        // env_logger::init();
        let data_provider = CacheProvider::default();
        test_server(data_provider).await;
    }

    #[tokio::test]
    async fn test_server_with_redis() {
        // env_logger::init();
        let docker_cli = DockerCli::default();
        let image = Redis;

        let redis_container = docker_cli.run(image);

        // Get the port of the running Redis container
        let server_port = redis_container.get_host_port_ipv4(6379);

        let data_provider = RedisProvider::new(RedisProviderArgs {
            server_port,
            ..Default::default()
        })
        .unwrap();
        test_server(data_provider).await;
    }

    async fn test_server<T: DataProvider + Default + 'static>(mut data_provider: T) {
        // env_logger::builder()
        //     .is_test(true)
        //     .try_init()
        //     .expect("Failed to init logger");
        let game_id = Uuid::new_v4();

        let random_port = rand::random::<u16>();
        let mut server = WebSocketServer::new(
            WebSocketServer::<T>::DEFAULT_HOST.to_string(),
            random_port,
            data_provider.clone(),
        );
        // let mut server = WebSocketServer::from_env(data_provider.clone());
        let server_address = server.get_address();

        data_provider.create_game(Some(game_id)).unwrap();
        tokio::spawn(async move {
            server.start().await.unwrap();
        });

        // wait for server to start
        sleep(Duration::from_millis(100)).await;
        debug!("connecting to server");
        // connect to server
        match timeout(
            Duration::from_millis(1000),
            connect_async(format!("ws://{}/{}", server_address, game_id)),
        )
        .await
        .unwrap()
        {
            Err(e) => {
                panic!("Error connecting to server: {:?}", e);
            }
            Ok((ws_stream, _)) => {
                let (_write, mut read) = ws_stream.split();
                // write
                //     .send(Message::Text(
                //         serde_json::to_string(&IncommingMessage::Ping {}).unwrap(),
                //     ))
                //     .await
                //     .unwrap();
                debug!("reading messages");

                let msg = timeout(Duration::from_millis(1000), read.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                let msg = serde_json::from_str::<OutgoingMessage>(&msg.to_string()).unwrap();
                debug!("received first message via websocket: {:?}", msg);

                data_provider
                    .add_move(game_id, Move::new((0, 0), Player::X))
                    .unwrap();

                let msg = read.next().await.unwrap().unwrap();
                debug!("received second message via websocket: {:?}", msg);
                // TODO: currently in beta, but assert_matches would be really neat here.
                assert!(matches!(
                    serde_json::from_str::<OutgoingMessage>(&msg.to_string()),
                    Ok(OutgoingMessage::GameState { game_state: _ })
                ))
            }
        }
    }
}
