pub use std::{fmt::Debug, future::Future};

use crate::DataProvider;
pub mod rest_api;
pub mod r#static;
pub mod websocket;

pub trait ServerArgs: Sized {
    fn from_env() -> Self;
}

pub trait Server<T: DataProvider + Default>: Sized {
    type ErrorKind: Debug;
    const DEFAULT_PORT: u16 = 3000;
    const DEFAULT_HOST: &'static str = "127.0.0.1";

    fn new(host: String, port: u16, data_provider: T) -> Self;

    fn start(&mut self) -> impl Future<Output = Result<(), Self::ErrorKind>> + Send;

    fn get_address(&self) -> String;

    fn default() -> Self;

    fn with_data_provider(data_provider: T) -> Self;

    // loads environment variables or uses default values if not set
    fn from_env(data_provider: T) -> Self;
}
