// mod client_management;
// mod data_provider;
// mod generic;

use log::info;
use tokio::{signal::ctrl_c, spawn};

use tictactoe_extreme::{
    r#static::StaticServer, rest_api::ApiServer, websocket::WebSocketServer, CacheProvider,
    DataProvider, RedisProvider, RedisProviderArgs, Server,
};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .is_test(true)
        .try_init()
        .expect("Failed to init logger");
    // read command line arguments
    let args: Vec<String> = std::env::args().collect();

    match args.get(1) {
        None => {
            let data_provider = CacheProvider::default();

            // all ports must be different from each other, if one is not set, none of the others can be 3000
            let ports = [
                std::env::var("WEBSERVER_PORT"),
                std::env::var("API_PORT"),
                std::env::var("WEBSOCKET_PORT"),
            ]
            .iter()
            .map(|val| val.clone().unwrap_or("3000".to_string()))
            .collect::<Vec<_>>();

            let unique_ports = ports.iter().collect::<std::collections::HashSet<_>>();
            if unique_ports.len() != ports.len() {
                panic!(
                    "All ports must be different from each other. Got: {:?}",
                    ports
                );
            }

            let mut static_server = StaticServer::from_env(data_provider.clone());
            let mut api_server = ApiServer::from_env(data_provider.clone());
            let mut websocket_server = WebSocketServer::from_env(data_provider.clone());
            spawn(async move {
                static_server.start().await.unwrap();
            });
            spawn(async move {
                api_server.start().await.unwrap();
            });
            spawn(async move {
                websocket_server.start().await.unwrap();
            });
        }
        Some(server) => {
            let data_provider = RedisProvider::new(RedisProviderArgs::from_env()).unwrap();
            match server.as_str() {
                "webserver" => {
                    // start webserver
                    info!("Starting webserver");
                    let mut static_server = StaticServer::from_env(data_provider.clone());
                    spawn(async move {
                        static_server.start().await.unwrap();
                    });
                }
                "api" => {
                    // start api server
                    info!("Starting api server");
                    let mut api_server = ApiServer::from_env(data_provider.clone());
                    spawn(async move {
                        api_server.start().await.unwrap();
                    });
                }
                "websocket" => {
                    // start websocket server
                    info!("Starting websocket server");
                    let mut websocket_server = WebSocketServer::from_env(data_provider.clone());
                    spawn(async move {
                        websocket_server.start().await.unwrap();
                    });
                }
                _ => {
                    panic!("Unknown server: {}", server);
                }
            }
        }
    }

    // wait for ctrl-c
    ctrl_c().await.unwrap();
}
