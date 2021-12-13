mod client;
mod connection;
mod errors;
mod query;
mod stream;
mod resource;
#[cfg(test)]
mod tests;

pub use client::Client;
pub use connection::Connection;
pub use errors::ClientError;
pub use query::{
    Query,
    ToQueryArgument,
    ArgumentWriter,
    serializer,
    analysis
};
pub use stream::DatabaseStream;

/// A [`Result`] with its [`Err`] variant set to [`ClientError`].
///
/// [`Result`]: std::result::Result
/// [`Err`]: std::result::Result::Err
/// [`ClientError`]: crate::errors::ClientError
pub type Result<T> = std::result::Result<T, ClientError>;
