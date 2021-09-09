use std::fmt::{Display, Formatter};
use std::io::Error;
use std::string::FromUtf8Error;

/// The error type for the DB operations of the [`Client`], [`Query`] and associated structs and traits.
///
/// Errors mostly occur while communicating with the database, but can also happen e.g. when parsing arguments.
///
/// [`Client`]: crate::client::Client
/// [`Query`]: crate::query::Query
#[derive(Debug)]
pub enum ClientError {
    /// The database connection stream or parsing arguments has resulted in an error.
    Io(Error),
    /// The byte sequence being parsed is not a valid UTF-8 sequence.
    Utf8Parse(FromUtf8Error),
    /// The provided credentials for authorizing are invalid.
    Auth,
    /// The command was processed but failed to get the expected result.
    CommandFailed {
        message: String,
    },
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &*self {
            ClientError::Io(ref e) => e.fmt(f),
            ClientError::Utf8Parse(ref e) => e.fmt(f),
            ClientError::Auth => write!(f, "Access denied."),
            ClientError::CommandFailed { message } => write!(f, "{}", message),
        }
    }
}

impl From<Error> for ClientError {
    fn from(err: Error) -> ClientError {
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
        let error = ClientError::Io(Error::new(ErrorKind::Other, "test"));
        let _ = format!("{:?}", error);
    }

    #[test]
    fn test_io_error_formats_as_empty() {
        let error = ClientError::Io(Error::new(ErrorKind::Other, "test"));
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
}
