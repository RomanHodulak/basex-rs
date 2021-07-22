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
pub trait DatabaseStream<'a>: Read + Write + Sized {
    fn try_clone(&'a mut self) -> Result<Self>;
}

impl DatabaseStream<'_> for TcpStream {
    fn try_clone(&mut self) -> Result<Self> {
        Ok(TcpStream::try_clone(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) struct MockStream<'a> {
        buffer: &'a mut Vec<u8>,
        response: String,
    }

    impl<'a> MockStream<'a> {
        pub(crate) fn new(buffer: &'a mut Vec<u8>, response: String) -> Self {
            Self { buffer, response }
        }
    }

    impl ToString for MockStream<'_> {
        fn to_string(&self) -> String {
            String::from_utf8(self.buffer.clone()).unwrap()
        }
    }

    impl Read for MockStream<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let size = self.response.as_bytes().len();
            (&mut *buf).write_all(self.response.as_bytes());
            (&mut *buf).write(&[0 as u8]);
            Ok(size)
        }
    }

    impl Write for MockStream<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let bytes_written = buf.len();
            self.buffer.extend(buf);
            Ok(bytes_written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            unimplemented!()
        }
    }

    impl<'a> DatabaseStream<'a> for MockStream<'a> {
        fn try_clone(&'a mut self) -> Result<Self> {
            Ok(MockStream::new(self.buffer, self.response.clone()))
        }
    }
}
