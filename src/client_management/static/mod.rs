use crate::{DataProvider, Server};
use actix_files::Files;
use actix_web::{App, HttpServer};
use log::debug;
use std::marker::PhantomData;

pub struct StaticServer<T> {
    pub port: u16,
    pub host: String,
    phantom: PhantomData<T>,
}

impl<T: DataProvider + Default> Server<T> for StaticServer<T> {
    type ErrorKind = std::io::Error;
    fn default() -> Self {
        Self {
            port: Self::DEFAULT_PORT,
            host: Self::DEFAULT_HOST.to_string(),
            phantom: PhantomData,
        }
    }

    fn new(host: String, port: u16, _data_provider: T) -> Self {
        Self {
            port,
            host,
            phantom: PhantomData,
        }
    }

    fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn with_data_provider(_data_provider: T) -> Self {
        Self::default()
    }

    fn from_env(_data_provider: T) -> Self {
        debug!("env: {:?}", std::env::vars());
        let port =
            std::env::var("WEBSERVER_PORT").unwrap_or_else(|_| Self::DEFAULT_PORT.to_string());
        let host =
            std::env::var("WEBSERVER_HOST").unwrap_or_else(|_| Self::DEFAULT_HOST.to_string());
        let port = port.parse::<u16>().unwrap_or(Self::DEFAULT_PORT);
        Self::new(host, port, T::default())
    }
    async fn start(&mut self) -> Result<(), Self::ErrorKind> {
        debug!("Starting static server on {}", self.get_address());
        HttpServer::new(|| App::new().service(Files::new("/", "./static").index_file("index.html")))
            .bind(self.get_address())
            .unwrap()
            .run()
            .await
    }
}

#[cfg(test)]
mod test {
    use reqwest::{Client, StatusCode};

    use tokio::{
        fs::write,
        spawn,
        time::{sleep, Duration},
    };

    use crate::CacheProvider;

    use super::*;

    #[tokio::test]
    async fn test_server() {
        let random_port = rand::random::<u16>();
        let default_host = StaticServer::<CacheProvider>::DEFAULT_HOST.to_string();
        let mut server =
            StaticServer::<CacheProvider>::new(default_host, random_port, CacheProvider::default());
        // create a file in the static folder
        let contents = "this is a test. Please delete this file";
        let address = server.get_address();
        write("./static/test.txt", contents).await.unwrap();
        spawn(async move {
            server.start().await.unwrap();
        });

        // wait for server to start
        sleep(Duration::from_secs(1)).await;

        let client = Client::new();
        let response = client
            .get(format!("http://{}/test.txt", address))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.text().await.unwrap();
        assert_eq!(body, contents);

        // delete the file
        tokio::fs::remove_file("./static/test.txt").await.unwrap();
    }
}
