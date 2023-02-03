pub(crate) mod client;
pub(crate) mod connection;
pub(crate) mod escape_reader;
pub(crate) mod query;
pub(crate) mod resource;
pub(crate) mod response;
#[cfg(test)]
pub(crate) mod tests;

pub use client::{Client, CommandWithOptionalInput, QueryWithOptionalInfo};
pub use connection::{Authenticated, Connection, ConnectionError, Unauthenticated};
pub use query::{
    compiler, serializer, ArgumentWithOptionalValue, ArgumentWriter, Query, QueryFailed, Response as QueryResponse,
    ToQueryArgument, WithInfo, WithoutInfo,
};
pub use resource::AsResource;
pub use response::Response as CommandResponse;
