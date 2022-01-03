use crate::Result;
use std::io::{Read, Write};
use std::net::TcpStream;

/// Represents an r/w communication channel with a [BaseX] server.
///
/// The BaseX [`Connection`] requires also a clone method that creates a copy of itself but is expected to reference
/// the same stream.
///
/// [`Connection`]: crate::connection::Connection
/// [BaseX]: http://basex.org
pub trait Stream: Read + Write + Sized {
    /// Creates a new independently owned handle to the underlying stream.
    fn try_clone(&self) -> Result<Self>;
}

impl Stream for TcpStream {
    fn try_clone(&self) -> Result<Self> {
        Ok(TcpStream::try_clone(self)?)
    }
}
