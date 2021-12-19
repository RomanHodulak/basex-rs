#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(unused)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![warn(rustdoc::private_doc_tests)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::missing_doc_code_examples)]
#![warn(rustdoc::invalid_codeblock_attributes)]
#![warn(rustdoc::invalid_html_tags)]
#![warn(rustdoc::invalid_rust_codeblocks)]
#![warn(rustdoc::bare_urls)]
mod client;
mod connection;
mod errors;
mod query;
mod resource;
mod stream;
#[cfg(test)]
mod tests;

pub use client::Client;
pub use connection::Connection;
pub use errors::ClientError;
pub use query::{compiler, serializer, ArgumentWriter, Query, ToQueryArgument, WithInfo, WithoutInfo};
pub use stream::DatabaseStream;

/// A [`Result`] with its [`Err`] variant set to [`ClientError`].
///
/// [`Result`]: std::result::Result
/// [`Err`]: std::result::Result::Err
/// [`ClientError`]: crate::errors::ClientError
pub type Result<T> = std::result::Result<T, ClientError>;
