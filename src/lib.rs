mod client;
mod connection;
mod errors;
mod query;
mod stream;
mod tests;

pub use client::Client;
pub use connection::Connection;
pub use errors::ClientError;
pub use query::Query;
pub use stream::DatabaseStream;

pub type Result<T> = std::result::Result<T, ClientError>;
