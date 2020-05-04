mod client;
mod connection;
mod errors;
mod query;

use std::net::TcpStream;
use std::io::{Write, Read, Error};
use std::string::FromUtf8Error;
use std::fmt::{Display, Formatter};

pub use client::Client;
pub use connection::Connection;
pub use errors::ClientError;
pub use query::Query;

pub type Result<T> = std::result::Result<T, ClientError>;

/// Connects and authenticates to BaseX server.
pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client> {
    let mut stream = TcpStream::connect(&format!("{}:{}", host, port))?;
    let mut connection = Connection::new(stream);
    connection.authenticate(user, password)?;

    Ok(Client::new(connection))
}
