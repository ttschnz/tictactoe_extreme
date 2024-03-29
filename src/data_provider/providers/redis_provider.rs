use crate::{Board, DataProvider, GameData, Move};

use log::debug;
use redis::Client;
use redis_async::{client::pubsub::pubsub_connect, resp::FromResp};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use tokio_stream::StreamExt;
use uuid::Uuid;

#[derive(Clone)]
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
            server_hostname: Self::DEFAULT_SERVER_HOSTNAME.to_string(),
            server_port: Self::DEFAULT_SERVER_PORT,
            username: None,
            password: None,
        }
    }

    pub fn from_env() -> Self {
        let server_hostname = std::env::var("REDIS_SERVER_HOSTNAME")
            .unwrap_or(Self::DEFAULT_SERVER_HOSTNAME.to_string());
        let server_port = std::env::var("REDIS_SERVER_PORT")
            .unwrap_or(Self::DEFAULT_SERVER_PORT.to_string())
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

impl Default for RedisProvider {
    fn default() -> Self {
        Self::new(RedisProviderArgs::default()).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ErrorKind {
    Connection { message: String },
    Query { message: String },
    Deserialize { message: String },
    Serialize { message: String },
}

impl ToString for ErrorKind {
    fn to_string(&self) -> String {
        match self {
            Self::Connection { message } => format!(
                "the connection to redis could not be established: {}",
                message
            ),
            Self::Deserialize { message } => {
                format!("the string from redis could not be serialized: {}", message)
            }
            Self::Query { message } => format!("there was an error querying redis: {}", message),
            Self::Serialize { message } => {
                format!("the local object could not be serialized: {}", message)
            }
        }
    }
}

impl RedisProvider {
    fn get_connection(&self) -> Result<redis::Connection, ErrorKind> {
        self.redis_client
            .get_connection()
            .map_err(|e| ErrorKind::Connection {
                message: format!("{}", e),
            })
    }
}

impl DataProvider for RedisProvider {
    type Args = RedisProviderArgs;
    type ErrorKind = ErrorKind;
    fn get_game_data(&self, game_id: Uuid) -> Result<GameData, ErrorKind> {
        debug!("Getting game data for game {}", game_id);
        let mut connection = self.get_connection()?;
        let remote_move_count = (redis::cmd("JSON.ARRLEN")
            .arg(game_id.to_string())
            .arg("$.moves")
            .query(&mut connection) as Result<Vec<usize>, _>)
            .map_err(|e| ErrorKind::Query {
                message: format!("{}", e),
            })?
            .remove(0);

        if remote_move_count == 0 {
            return Ok(GameData::new_with_id(game_id));
        }

        let serialized_game: String = redis::cmd("JSON.GET")
            .arg(game_id.to_string())
            .query(&mut connection)
            .map_err(|e| ErrorKind::Query {
                message: format!("{}", e),
            })?;

        debug!("Deserializing game data: {}", serialized_game);
        let game_data: GameData =
            from_str(&serialized_game).map_err(|e| ErrorKind::Deserialize {
                message: format!("{}", e),
            })?;

        Ok(game_data)
    }

    fn get_games(&self) -> Result<Vec<Uuid>, Self::ErrorKind> {
        let mut connection = self.get_connection()?;
        let game_ids: Vec<String> =
            redis::cmd("KEYS")
                .arg("*")
                .query(&mut connection)
                .map_err(|e| ErrorKind::Query {
                    message: format!("{}", e),
                })?;

        let game_ids: Vec<Uuid> = game_ids
            .into_iter()
            .filter_map(|x| Uuid::parse_str(&x).ok())
            .collect();

        Ok(game_ids)
    }

    fn game_exists(&mut self, game_id: Uuid) -> Result<bool, Self::ErrorKind> {
        let mut connection = self.get_connection()?;

        let exists: bool = redis::cmd("EXISTS")
            .arg(game_id.to_string())
            .query(&mut connection)
            .map_err(|e| ErrorKind::Query {
                message: format!("{}", e),
            })?;

        Ok(exists)
    }

    fn add_move(&mut self, game_id: Uuid, new_move: Move) -> Result<(), ErrorKind> {
        let mut connection = self.get_connection()?;

        let stringified_move = to_string(&new_move).map_err(|e| ErrorKind::Serialize {
            message: format!("{}", e),
        })?;

        redis::cmd("JSON.ARRAPPEND")
            .arg(game_id.to_string())
            .arg("$.moves")
            .arg(stringified_move)
            .query(&mut connection)
            .map_err(|e| ErrorKind::Query {
                message: format!("{}", e),
            })?;

        debug!("Added move {:?} to game {}", new_move, game_id);

        debug!("Publishing game data to channel {}", game_id);
        let game_data = self.get_game_data(game_id)?;
        let serialized_game_data = to_string(&game_data).map_err(|e| ErrorKind::Serialize {
            message: format!("{}", e),
        })?;

        redis::cmd("PUBLISH")
            .arg(game_id.to_string())
            .arg(serialized_game_data)
            .query(&mut connection)
            .map_err(|e| ErrorKind::Query {
                message: format!("{}", e),
            })?;

        Ok(())
    }

    fn create_game(&mut self, uuid: Option<Uuid>) -> Result<Uuid, ErrorKind> {
        let mut connection = self.get_connection()?;
        let uuid = uuid.unwrap_or(Uuid::new_v4());

        let game = GameData::new_with_id(uuid);

        let serialized_game = to_string(&game).map_err(|e| ErrorKind::Serialize {
            message: format!("{}", e),
        })?;

        redis::cmd("JSON.SET")
            .arg(uuid.to_string())
            .arg("$")
            .arg(serialized_game)
            .query(&mut connection)
            .map_err(|e| ErrorKind::Query {
                message: format!("{}", e),
            })?;

        debug!("Created game {}", uuid);
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

    fn sync_board(&mut self, game: &mut Board) -> Result<(), Self::ErrorKind> {
        debug!("Syncing board {}", game.game_id);

        // test if remote game data exists
        if self.get_game_data(game.game_id).is_err() {
            debug!(
                "Remote game data for {} doesn't exist. Creating...",
                game.game_id
            );
            self.create_game(Some(game.game_id))?;
        }

        let mut local_game_data: GameData = game.clone().into();
        let mut remote_game_data = self.get_game_data(game.game_id)?;

        let mut moves_to_upload = Vec::new();

        if local_game_data != remote_game_data {
            debug!(
                "Difference between local and remote game data {} detected. Syncing...",
                game.game_id
            );
            let local_moves = &mut local_game_data.moves;
            let remote_moves = &mut remote_game_data.moves;
            // compare each move
            for move_index in 0..local_moves.len().max(remote_moves.len()) {
                // does the move not exist in any of the game data?
                if move_index >= local_moves.len() {
                    debug!("Adding remote move {} to local game data", move_index);
                    local_moves.push(remote_moves[move_index]);
                    continue;
                }
                if move_index >= remote_moves.len() {
                    debug!("Adding local move {} to remote game data", move_index);
                    moves_to_upload.push(local_moves[move_index]);
                    continue;
                }

                // the move exists in both game data. remote has priority
                debug!(
                    "Conflict detected at move {}. Prioritizing remote move",
                    move_index
                );
                local_moves[move_index] = remote_moves[move_index];
            }

            // update local game data
            *game = local_game_data.into();

            // upload moves
            debug!(
                "Uploading {} moves to remote game data",
                moves_to_upload.len()
            );
            for new_move in moves_to_upload {
                debug!("Uploading move {:?} to remote game data", new_move);
                self.add_move(game.game_id, new_move)?;
            }
        }

        Ok(())
    }

    fn subscribe_to_game(
        &mut self,
        game_id: Uuid,
    ) -> Result<tokio::sync::watch::Receiver<GameData>, Self::ErrorKind> {
        // TODO: This is a very naive implementation. It should be thoroughly tested

        debug!("Subscribing to game {}", game_id);
        // let mut connection = self.get_connection()?;
        let (tx, rx) = tokio::sync::watch::channel(GameData::new_with_id(game_id));
        let args = self._args.clone();
        tokio::spawn(async move {
            let connection = pubsub_connect(args.server_hostname, args.server_port)
                .await
                .unwrap();

            // let mut pubsub = connection.as_pubsub();
            let mut stream = connection.subscribe(&game_id.to_string()).await.unwrap();

            while let Some(Ok(msg)) = stream.next().await {
                let msg = String::from_resp(msg).unwrap();
                debug!("Received pubsub message: {:?}", msg);
                let game_data: GameData = from_str(&msg).unwrap();
                debug!("Sending new game data to subscribers: {:?}", game_data);
                tx.send(game_data).unwrap();
            }
            // loop {
            //     let msg = pubsub.get_message().unwrap();
            //     debug!("Received pubsub message: {:?}", msg);
            //     let payload: String = msg.get_payload().unwrap();
            //     let game_data: GameData = from_str(&payload).unwrap();
            //     debug!("Sending new game data to subscribers: {:?}", game_data);
            //     tx.send(game_data).unwrap();

            //     // debug:
            //     // for _ in 0..5 {
            //     //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            //     //     let game_data = GameData::default();
            //     //     tx.send(game_data).unwrap()
            //     // }
            // }
        });

        Ok(rx)
    }
}

#[cfg(test)]
pub mod test {

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
    use crate::{DataProviderFactory, Player};

    use redis::Client;
    use redis_stack::Redis;
    use testcontainers::clients::Cli as DockerCli;

    #[tokio::test]
    async fn start_redis_server() {
        let docker_cli = DockerCli::default();
        let image = Redis;

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
        // std::env::set_var("RUST_LOG", "debug");
        // env_logger::init();

        let docker_cli = DockerCli::default();
        let image = Redis;

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

        // test insert move
        {
            let uuid = data_provider
                .create_game(None)
                .expect("Failed to create game");

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

        debug!("starting sync tests");
        // test sync Local -> Remote
        {
            let uuid = data_provider
                .create_game(None)
                .expect("Failed to create game");
            let mut board = Board::from(
                data_provider
                    .get_game_data(uuid)
                    .expect("Failed to get game data"),
            );
            board
                .insert_move((0, 0), Player::X)
                .expect("Failed to insert move");

            data_provider
                .sync_board(&mut board)
                .expect("Failed to sync board");

            let remote_game_data = data_provider
                .get_game_data(uuid)
                .expect("Failed to get game data");

            assert_eq!(
                board.moves, remote_game_data.moves,
                "Remote game data didn't update to match local game data: {:?}",
                remote_game_data.moves
            );

            // test sync Remote -> Local
            let moves = remote_game_data.moves.clone();
            let mut board = Board {
                game_id: uuid,
                ..Default::default()
            };

            data_provider
                .sync_board(&mut board)
                .expect("Failed to sync board");

            let remote_game_data = data_provider.get_game_data(uuid).unwrap();

            assert_eq!(
                board.moves, moves,
                "Local game data didn't sync correctly: {:?}",
                board.moves
            );
            assert_eq!(
                remote_game_data.moves, moves,
                "Remote game data changed during sync: {:?}",
                remote_game_data
            );

            // test sync Remote -> Local with conflict: Remote has priority
            let mut board = Board {
                game_id: uuid,
                ..Default::default()
            };

            board
                .insert_move((4, 4), Player::X)
                .expect("Failed to insert move");

            data_provider
                .sync_board(&mut board)
                .expect("Failed to sync board");

            let remote_game_data = data_provider.get_game_data(uuid).unwrap();

            assert_eq!(
                board.moves, moves,
                "Local game data didn't sync correctly during conflict sync: {:?}",
                board.moves
            );
            assert_eq!(
                remote_game_data.moves, moves,
                "Remote game data changed during conflict sync: {:?}",
                remote_game_data
            );
        }

        {
            // sync unexisting game

            let mut board = Board {
                game_id: Uuid::new_v4(),
                ..Default::default()
            };

            data_provider
                .sync_board(&mut board)
                .expect("Failed to sync board");

            let remote_game_data = data_provider
                .get_game_data(board.game_id)
                .expect("Failed to get game data");

            assert_eq!(
                board.moves, remote_game_data.moves,
                "Remote game data didn't update to match local game data: {:?}",
                remote_game_data.moves
            );
        }
    }
}
