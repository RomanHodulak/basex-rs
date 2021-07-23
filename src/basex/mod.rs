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
pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client<TcpStream>> {
    let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
    let mut connection = Connection::new(stream);
    connection.authenticate(user, password)?;

    Ok(Client::new(connection))
}

/// Represents a stream usable for BaseX database connection.
///
/// The BaseX connection requires r/w stream and also a clone method that creates a copy of itself
/// but is expected to reference the same stream.
pub trait DatabaseStream: Read + Write + Sized {
    fn try_clone(&mut self) -> Result<Self>;
}

impl DatabaseStream for TcpStream {
    fn try_clone(&mut self) -> Result<Self> {
        Ok(TcpStream::try_clone(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;

    pub(crate) struct MockStream {
        buffer: Rc<RefCell<Vec<u8>>>,
        response: String,
    }

    impl MockStream {
        pub(crate) fn new(response: String) -> Self {
            Self { buffer: Rc::new(RefCell::new(vec![])), response }
        }
    }

    impl ToString for MockStream {
        fn to_string(&self) -> String {
            String::from_utf8(self.buffer.borrow().clone()).unwrap()
        }
    }

    impl Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let size = self.response.as_bytes().len();
            (&mut *buf).write_all(self.response.as_bytes());
            (&mut *buf).write(&[0 as u8]);
            Ok(size)
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let bytes_written = buf.len();
            self.buffer.borrow_mut().extend(buf);
            Ok(bytes_written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            unimplemented!()
        }
    }

    impl DatabaseStream for MockStream {
        fn try_clone(&mut self) -> Result<Self> {
            Ok(MockStream {
                buffer: Rc::clone(&self.buffer),
                response: self.response.clone()
            })
        }
    }
}
