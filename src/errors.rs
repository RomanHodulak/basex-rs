use std::fmt::{Display, Formatter};
use std::io;
use std::error;
use std::string::FromUtf8Error;
use crate::query::QueryFailed;

/// The error type for the DB operations of the [`Client`], [`Query`] and associated structs and traits.
///
/// Errors mostly occur while communicating with the database, but can also happen e.g. when parsing arguments.
///
/// [`Client`]: crate::client::Client
/// [`Query`]: crate::query::Query
#[derive(Debug)]
pub enum ClientError {
    /// The database connection stream or parsing arguments has resulted in an error.
    Io(io::Error),
    /// The byte sequence being parsed is not a valid UTF-8 sequence.
    Utf8Parse(FromUtf8Error),
    /// The provided credentials for authorizing are invalid.
    Auth,
    /// The command was processed but failed to get the expected result.
    CommandFailed {
        message: String,
    },
    /// The query was processed but failed to get the expected result.
    QueryFailed(QueryFailed),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &*self {
            ClientError::Io(ref e) => e.fmt(f),
            ClientError::Utf8Parse(ref e) => e.fmt(f),
            ClientError::Auth => write!(f, "access denied"),
            ClientError::CommandFailed { message } => write!(f, "{}", message),
            ClientError::QueryFailed(q) => write!(f, "{}", q.raw()),
        }
    }
}

impl error::Error for ClientError {
}

impl From<io::Error> for ClientError {
    fn from(err: io::Error) -> ClientError {
        ClientError::Io(err)
    }
}

impl From<FromUtf8Error> for ClientError {
    fn from(err: FromUtf8Error) -> ClientError {
        ClientError::Utf8Parse(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn test_io_error_formats_as_debug() {
        let error = ClientError::Io(io::Error::new(ErrorKind::Other, "test"));
        let _ = format!("{:?}", error);
    }

    #[test]
    fn test_io_error_formats_as_empty() {
        let error = ClientError::Io(io::Error::new(ErrorKind::Other, "test"));
        let _ = format!("{}", error);
    }

    #[test]
    fn test_utf8_parse_formats_as_debug() {
        let error = ClientError::Utf8Parse(String::from_utf8(vec![0xa0 as u8, 0xa1]).unwrap_err());
        let _ = format!("{:?}", error);
    }

    #[test]
    fn test_utf8_parse_formats_as_empty() {
        let error = ClientError::Utf8Parse(String::from_utf8(vec![0xa0 as u8, 0xa1]).unwrap_err());
        let _ = format!("{}", error);
    }

    #[test]
    fn test_auth_formats_as_debug() {
        let error = ClientError::Auth;
        let _ = format!("{:?}", error);
    }

    #[test]
    fn test_auth_formats_as_empty() {
        let error = ClientError::Auth;
        let _ = format!("{}", error);
    }

    #[test]
    fn test_command_failed_formats_as_debug() {
        let error = ClientError::CommandFailed { message: "error".to_owned() };
        let _ = format!("{:?}", error);
    }

    #[test]
    fn test_command_failed_formats_as_empty() {
        let error = ClientError::CommandFailed { message: "error".to_owned() };
        let _ = format!("{}", error);
    }

    #[test]
    fn test_query_failed_formats_as_debug() {
        let error = ClientError::QueryFailed(QueryFailed::new(
            "Stopped at ., 1/1: [XPST0008] Undeclared variable $x.".to_owned()
        ));
        let _ = format!("{:?}", error);
    }

    #[test]
    fn test_query_failed_formats_as_empty() {
        let error = ClientError::QueryFailed(QueryFailed::new(
            "Stopped at ., 1/1: [XPST0008] Undeclared variable $x.".to_owned()
        ));
        let _ = format!("{}", error);
    }
}
