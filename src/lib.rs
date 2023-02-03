//! Client implementation of the open source XML database server and XQuery processor [BaseX].
//!
//! # Examples
//! The following example creates database "lambada" with initial XML resource and counts all first-level child nodes
//! of the `Root` node.
//! ```
//! use basex::{Client, ClientError};
//! use std::io::Read;
//!
//! fn main() -> Result<(), ClientError> {
//!     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
//!     let info = client.create("lambada")?
//!         .with_input("<Root><Text/><Lala/><Papa/></Root>")?;
//!     assert!(info.starts_with("Database 'lambada' created"));
//!
//!     let query = client.query("count(/Root/*)")?.without_info()?;
//!
//!     let mut result = String::new();
//!     let mut response = query.execute()?;
//!     response.read_to_string(&mut result)?;
//!     assert_eq!(result, "3");
//!
//!     let mut query = response.close()?;
//!     query.close()?;
//!     Ok(())
//! }
//! ```
//!
//! [BaseX]: http://basex.org

#![warn(rust_2018_idioms)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(unused)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::missing_doc_code_examples)]
#![warn(rustdoc::invalid_codeblock_attributes)]
#![warn(rustdoc::invalid_html_tags)]
#![warn(rustdoc::invalid_rust_codeblocks)]
#![warn(rustdoc::bare_urls)]

pub mod asynchronous;
mod client;
mod connection;
mod errors;
mod query;
mod resource;
mod stream;
#[cfg(test)]
mod tests;

pub use client::{Client, CommandWithOptionalInput, QueryWithOptionalInfo, Response as CommandResponse};
pub use connection::{Authenticated, Connection, Unauthenticated};
pub use errors::ClientError;
pub use query::{
    compiler, serializer, ArgumentWithOptionalValue, ArgumentWriter, Query, QueryFailed, Response as QueryResponse,
    ToQueryArgument, WithInfo, WithoutInfo,
};
pub use resource::AsResource;
pub use stream::Stream;

/// A [`Result`] with its [`Err`] variant set to [`ClientError`].
///
/// [`Result`]: std::result::Result
/// [`Err`]: std::result::Result::Err
/// [`ClientError`]: crate::errors::ClientError
pub type Result<T> = std::result::Result<T, ClientError>;
