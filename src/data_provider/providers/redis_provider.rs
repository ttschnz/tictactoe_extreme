use crate::{DataProvider, GameData, Move};

use redis::Client;
use serde_json::{from_str, to_string};
use uuid::Uuid;

pub struct RedisProvider {
    _args: RedisProviderArgs,

    redis_client: Client,
}

#[derive(Clone)]
pub struct RedisProviderArgs {
    pub server_hostname: String,
    pub server_port: u16,

    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for RedisProviderArgs {
    fn default() -> Self {
        Self::new()
    }
}

impl RedisProviderArgs {
    const DEFAULT_SERVER_HOSTNAME: &'static str = "localhost";
    const DEFAULT_SERVER_PORT: u16 = 6379;

    pub fn new() -> Self {
        Self {
            server_hostname: RedisProviderArgs::DEFAULT_SERVER_HOSTNAME.to_string(),
            server_port: RedisProviderArgs::DEFAULT_SERVER_PORT,
            username: None,
            password: None,
        }
    }

    pub fn from_env() -> Self {
        let server_hostname = std::env::var("REDIS_SERVER_HOSTNAME")
            .unwrap_or(RedisProviderArgs::DEFAULT_SERVER_HOSTNAME.to_string());
        let server_port = std::env::var("REDIS_SERVER_PORT")
            .unwrap_or(RedisProviderArgs::DEFAULT_SERVER_PORT.to_string())
            .parse::<u16>()
            .expect("Failed to parse REDIS_SERVER_PORT");

        let username = std::env::var("REDIS_USERNAME").ok();
        let password = std::env::var("REDIS_PASSWORD").ok();

        Self {
            server_hostname,
            server_port,
            username,
            password,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    ConnectionError { message: String },
    QueryError { message: String },
    DeserializeError { message: String },
    SerializeError { message: String },
}

impl RedisProvider {
    fn get_connection(&self) -> Result<redis::Connection, ErrorKind> {
        self.redis_client
            .get_connection()
            .map_err(|e| ErrorKind::ConnectionError {
                message: format!("{}", e),
            })
    }
}

impl DataProvider for RedisProvider {
    type Args = RedisProviderArgs;
    type ErrorKind = ErrorKind;
    fn get_game_data(&self, game_id: Uuid) -> Result<GameData, ErrorKind> {
        let mut connection = self.get_connection()?;
        let serialized_game: String = redis::cmd("JSON.GET")
            .arg(game_id.to_string())
            .query(&mut connection)
            .map_err(|e| ErrorKind::QueryError {
                message: format!("{}", e),
            })?;

        let game_data: GameData =
            from_str(&serialized_game).map_err(|e| ErrorKind::DeserializeError {
                message: format!("{}", e),
            })?;

        Ok(game_data)
    }

    fn add_move(&mut self, game_id: Uuid, new_move: Move) -> Result<(), ErrorKind> {
        let mut connection = self.get_connection()?;

        let stringified_move = to_string(&new_move).map_err(|e| ErrorKind::SerializeError {
            message: format!("{}", e),
        })?;

        redis::cmd("JSON.ARRAPPEND")
            .arg(game_id.to_string())
            .arg("$.moves")
            .arg(stringified_move)
            .query(&mut connection)
            .map_err(|e| ErrorKind::QueryError {
                message: format!("{}", e),
            })?;

        Ok(())
    }

    fn create_game(&mut self) -> Result<Uuid, ErrorKind> {
        let mut connection = self.get_connection()?;
        let uuid = Uuid::new_v4();

        let game = GameData::new();

        let serialized_game = to_string(&game).map_err(|e| ErrorKind::SerializeError {
            message: format!("{}", e),
        })?;

        redis::cmd("JSON.SET")
            .arg(uuid.to_string())
            .arg("$")
            .arg(serialized_game)
            .query(&mut connection)
            .map_err(|e| ErrorKind::QueryError {
                message: format!("{}", e),
            })?;

        Ok(uuid)
    }

    fn new(args: Self::Args) -> Result<Self, ErrorKind> {
        let redis_client = Client::open(format!(
            "redis://{}:{}",
            args.server_hostname, args.server_port
        ))
        .expect("Failed to create Redis client");
        Ok(Self {
            _args: args.clone(),
            redis_client,
        })
    }
}

#[cfg(test)]
mod test {

    mod redis_stack {
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
    use crate::{DataProviderFactory, Player};
    use redis::Client;
    use redis_stack::Redis;
    use testcontainers::clients::Cli as DockerCli;

    #[tokio::test]
    async fn start_redis_server() -> () {
        let docker_cli = DockerCli::default();
        let image = Redis::default();

        let redis_container = docker_cli.run(image);

        // Get the port of the running Redis container
        let redis_port = redis_container.get_host_port_ipv4(6379);

        // Build a Redis client
        let redis_url = format!("redis://{}:{}", "localhost", redis_port);

        let client = Client::open(redis_url).expect("Failed to create Redis client");

        // Perform some Redis operations
        let mut connection = client.get_connection().unwrap();

        let _: () = redis::cmd("SET")
            .arg("key")
            .arg("value")
            .query(&mut connection)
            .expect("Failed to set key");

        let result: String = redis::cmd("GET")
            .arg("key")
            .query(&mut connection)
            .expect("Failed to get key");
        assert_eq!(result, "value");

        // Clean up: Stop the Redis container
        redis_container.stop();
    }

    #[tokio::test]
    async fn test_data_storage() {
        let docker_cli = DockerCli::default();
        let image = Redis::default();

        let redis_container = docker_cli.run(image);

        // Get the port of the running Redis container
        let redis_port = redis_container.get_host_port_ipv4(6379);

        let args = RedisProviderArgs {
            server_hostname: "localhost".to_string(),
            server_port: redis_port,
            username: None,
            password: None,
        };

        let mut data_provider = DataProviderFactory::create::<RedisProvider>(args)
            .expect("Failed to create RedisProvider");

        let uuid = data_provider.create_game().expect("Failed to create game");

        let mut local_game_data = data_provider
            .get_game_data(uuid)
            .expect("Failed to get game data");

        let new_move_1 = Move::new((1, 1), Player::X);

        data_provider
            .add_move(uuid, new_move_1)
            .expect("Failed to add move");
        local_game_data.add_move(new_move_1);

        let new_move_2 = Move::new((2, 2), Player::X);

        data_provider
            .add_move(uuid, new_move_2)
            .expect("Failed to add move");

        local_game_data.add_move(new_move_2);

        let remote_game_data = data_provider
            .get_game_data(uuid)
            .expect("Failed to get game data");

        assert_eq!(local_game_data, remote_game_data);
    }
}
