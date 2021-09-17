use std::io::{Write, Read};
use std::net::TcpStream;
use crate::Result;

/// Represents a stream usable for BaseX database connection.
///
/// The BaseX connection requires r/w stream and also a clone method that creates a copy of itself
/// but is expected to reference the same stream.
pub trait DatabaseStream: Read + Write + Sized {
    /// Creates a new independently owned handle to the underlying stream.
    ///
    /// The returned instance is a reference to the same stream that this object references. Both handles will read and
    /// write the same stream of data, and options set on one stream will be propagated to the other stream.
    fn try_clone(&mut self) -> Result<Self>;
}

impl DatabaseStream for TcpStream {
    fn try_clone(&mut self) -> Result<Self> {
        Ok(TcpStream::try_clone(self)?)
    }
}
