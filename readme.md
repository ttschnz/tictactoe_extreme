# Tictactoe Extreme

Tictactoe Extreme is a rust-written fullstack application which can be used to play an extreme version of tictactoe against an opponent

## Installation

Currently it is not possible to install or start Tictactoe Extreme via any package manager.
You can however `clone` it and then run it.

```bash
git clone https://github.com/ttschnz/tictactoe_extreme
```
You can then run it directly, or via docker
### Direct run
Required for cargo to be installed. See [rustup](https://rustup.rs/).

This method is more for debugging than anything else. The three services (websocket, api, static server) are started on their own ports and hosts, given by the environment variables:
- `WEBSERVER_PORT` and `WEBSERVER_HOST`
- `WEBSOCKET_PORT` and `WEBSOCKET_HOST`
- `API_PORT` and `API_HOST`
On linux you can do it with the following command:
```bash
export WEBSERVER_PORT=3000
export WEBSOCKET_PORT=4000
export API_PORT=5000
export WEBSERVER_HOST=[::]
export WEBSOCKET_HOST=[::]
export API_HOST=[::]
```

then build and run the service you'd like:
```bash
cargo run --release -- [api|webserver|websocket|'']
```

then build the 
### Docker
```bash
docker compose up -d && docker compose logs -f 
```

## Usage

this crate gives you access to a library, you can use it according to:
<!-- //TODO! -->

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

[MIT](https://choosealicense.com/licenses/mit/)