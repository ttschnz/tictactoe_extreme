pub use std::{fmt::Debug, future::Future};
pub mod rest_api;
pub mod websocket;

pub trait ServerArgs: Sized {
    fn from_env() -> Self;
}

pub trait Server<T>: Sized {
    type Args: Clone + ServerArgs;
    type ErrorKind: Debug;

    fn new(config: Self::Args) -> Result<Self, Self::ErrorKind>;

    fn start(
        &mut self,
        data_provider: T,
    ) -> impl Future<Output = Result<(), Self::ErrorKind>> + Send;
}
