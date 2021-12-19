#[allow(clippy::module_inception)]
mod connection;
mod escape_reader;

pub use self::connection::Authenticated;
pub use self::connection::Connection;
pub use self::connection::Unauthenticated;

pub(crate) use self::connection::HasConnection;
