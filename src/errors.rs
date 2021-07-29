use std::fmt::{Display, Formatter};
use std::io::Error;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum ClientError {
    Io(Error),
    Utf8Parse(FromUtf8Error),
    Auth,
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
