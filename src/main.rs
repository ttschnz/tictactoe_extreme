pub mod data_provider;
pub mod generic;

use data_provider::*;
use generic::*;


use log::info;

fn main() {
    env_logger::init();

    let mut redis_provider =
        DataProviderFactory::create::<RedisProvider>(RedisProviderArgs::from_env()).unwrap();

    let uuid = redis_provider.create_game().unwrap();
    info!("Created game with uuid: {}", uuid);

    let mut game_data = redis_provider.get_game_data(uuid).unwrap();
    info!("Game data: {:?}", game_data);

    let new_move = Move::new((1, 2), Player::X);
    game_data.add_move(new_move);
    info!("Game data: {:?}", game_data);

    redis_provider
        .add_move(uuid, new_move)
        .expect("Failed to add move");

    info!("Remote game data: {:?}", game_data);
}
