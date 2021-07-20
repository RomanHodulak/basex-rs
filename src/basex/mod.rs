mod client;
mod connection;
mod errors;
mod query;

use std::net::TcpStream;

pub use client::Client;
pub use connection::Connection;
pub use errors::ClientError;
pub use query::Query;
use std::io::{Write, Read};

pub type Result<T> = std::result::Result<T, ClientError>;

/// Connects and authenticates to BaseX server.
pub fn connect<'a>(host: &str, port: u16, user: &str, password: &str) -> Result<Client<'a, TcpStream>> {
    let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
    let mut connection = Connection::new(stream);
    connection.authenticate(user, password)?;

    Ok(Client::new(connection))
}

/// Represents a stream usable for BaseX database connection.
///
/// The BaseX connection requires r/w stream and also a clone method that creates a copy of itself
/// but is expected to reference the same stream.
pub trait BasexStream<'a>: Read + Write + Sized {
    fn try_clone(&'a mut self) -> Result<Self>;
}

impl BasexStream<'_> for TcpStream {
    fn try_clone(&mut self) -> Result<Self> {
        Ok(TcpStream::try_clone(self)?)
    }
}
