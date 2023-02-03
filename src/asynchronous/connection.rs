use pin_project::pin_project;
use std::error;
use std::fmt::{Display, Formatter};
use std::io;
use std::marker::PhantomData;
use std::pin::Pin;
use std::string::FromUtf8Error;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

#[derive(Debug)]
pub enum ConnectionError {
    /// The database connection stream or parsing arguments has resulted in an error.
    Io(io::Error),
    /// The byte sequence being parsed is not a valid UTF-8 sequence.
    Utf8Parse(FromUtf8Error),
    /// The provided credentials for authorizing are invalid.
    Auth,
    /// The command was processed but failed to get the expected result.
    CommandFailed(String),
    /// The query was processed but failed to get the expected result.
    QueryFailed(crate::asynchronous::query::QueryFailed),
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &*self {
            Self::Io(ref e) => e.fmt(f),
            Self::Utf8Parse(ref e) => e.fmt(f),
            Self::Auth => write!(f, "access denied"),
            Self::CommandFailed(message) => write!(f, "{}", message),
            Self::QueryFailed(q) => write!(f, "{}", q.raw()),
        }
    }
}

impl error::Error for ConnectionError {}

impl From<io::Error> for ConnectionError {
    fn from(value: io::Error) -> Self {
        ConnectionError::Io(value)
    }
}

impl From<FromUtf8Error> for ConnectionError {
    fn from(value: FromUtf8Error) -> Self {
        ConnectionError::Utf8Parse(value)
    }
}

type Result<T> = std::result::Result<T, ConnectionError>;

/// [`Connection`] state before authentication for the session.
///
/// [`Connection`]: crate::Connection
#[derive(Debug, Clone)]
pub struct Unauthenticated;

/// [`Connection`] state after successful authentication for the session.
///
/// [`Connection`]: crate::Connection
#[derive(Debug, Clone)]
pub struct Authenticated;

/// Responsible for low-level communication with the [`Stream`]. It handles [authentication], sends commands and
/// reads responses.
///
/// As opposed to the [`Client`] or [`Query`] can do, connection does not understand what commands do or how to parse
/// responses. It can only send them, send arguments and be read like a stream.
///
/// [`Client`]: crate::Client
/// [`Query`]: crate::Query
/// [`Stream`]: crate::Stream
/// [authentication]: https://docs.basex.org/wiki/Server_Protocol#Authentication
#[derive(Debug)]
#[pin_project]
pub struct Connection<T, State = Unauthenticated>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    state: PhantomData<State>,
    #[pin]
    stream: T,
}

impl<T> Connection<T, Unauthenticated>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    /// Creates a connection that communicates with the database via the provided `stream`.
    pub fn new(stream: T) -> Self {
        Self {
            state: PhantomData::default(),
            stream,
        }
    }

    /// Authenticates the connection using the
    /// [server protocol](https://docs.basex.org/wiki/Server_Protocol#Authentication). Being authenticated is the
    /// pre-requisite for every other method to work.
    ///
    /// # Arguments
    /// * `user`: Username.
    /// * `password`: Password.
    pub async fn authenticate(mut self, user: &str, password: &str) -> Result<Connection<T, Authenticated>> {
        let response = self.read_string().await?;

        let challenge: Vec<&str> = response.split(':').collect();
        let server_name = challenge[0];
        let timestamp = challenge[1];

        let first_digest = md5::compute(&format!("{}:{}:{}", user, server_name, password));
        let second_digest = md5::compute(&format!("{:x}{}", first_digest, timestamp));

        let auth_string = format!("{}\0{:x}\0", user, second_digest);
        let mut control_byte: [u8; 1] = [0];

        self.stream.write_all(auth_string.as_bytes()).await?;
        self.stream.read_exact(&mut control_byte).await?;

        if control_byte[0] != 0 {
            return Err(ConnectionError::Auth);
        }

        Ok(Connection {
            state: Default::default(),
            stream: self.stream,
        })
    }
}

impl<T> Connection<T, Authenticated>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    pub(crate) async fn send_cmd(&mut self, code: u8) -> Result<&mut Self> {
        self.stream.write_all(&[code]).await?;

        Ok(self)
    }

    pub(crate) async fn send_arg(&mut self, argument: &mut (impl AsyncRead + Unpin)) -> Result<&mut Self> {
        use crate::asynchronous::escape_reader::EscapeReader;
        tokio::io::copy(&mut EscapeReader::new(argument), &mut self.stream).await?;

        self.skip_arg().await
    }

    pub(crate) async fn skip_arg(&mut self) -> Result<&mut Self> {
        self.stream.write_all(&[0]).await?;

        Ok(self)
    }

    /// Gets response string, and returns string if command was successful. Returns `CommandFailed`
    /// error with a message otherwise.
    pub(crate) async fn get_response(&mut self) -> Result<String> {
        let info = self.read_string().await?;

        if self.is_ok().await? {
            Ok(info)
        } else {
            Err(ConnectionError::CommandFailed(info))
        }
    }

    /// Reads return code and decodes it to TRUE on success or FALSE on error.
    pub(crate) async fn is_ok(&mut self) -> Result<bool> {
        let mut buf: [u8; 1] = [0];
        self.stream.read_exact(&mut buf).await?;

        Ok(buf[0] == 0)
    }
}

impl<T, State> AsyncRead for Connection<T, State>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        match buf.capacity() == 0 {
            true => Poll::Ready(Ok(())),
            false => self.project().stream.poll_read(cx, buf),
        }
    }
}

impl<T, State> Connection<T, State>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    pub(crate) async fn read_string(&mut self) -> Result<String> {
        let mut raw_string: Vec<u8> = vec![];
        loop {
            let mut buf: [u8; 1] = [0];
            self.stream.read_exact(&mut buf).await?;

            if buf[0] == 0 {
                break;
            }
            raw_string.push(buf[0]);
        }

        Ok(String::from_utf8(raw_string)?)
    }
}

impl<T, State> Clone for Connection<T, State>
where
    T: AsyncWrite + AsyncRead + Unpin + Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: Default::default(),
            stream: self.stream.clone(),
        }
    }
}

/// Contains [`Connection`]. Call [`HasConnection::connection`] to get mutable handle to use [`Connection`].
///
/// [`Connection`]: self::Connection
/// [`HasConnection::connection`]: self::HasConnection::connection
pub(crate) trait HasConnection<T: AsyncRead + AsyncWrite + Unpin> {
    /// Returns mutable reference to wrapped [`Connection`]. This is useful to make low-level calls to the server
    /// protocol but can result in unexpected state changes when used improperly.
    ///
    /// [`Connection`]: self::Connection
    fn connection(self: Pin<&mut Self>) -> Pin<&mut Connection<T, Authenticated>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asynchronous::tests::{FailingStream, MockStream};

    impl<T, State> Connection<T, State>
    where
        T: AsyncWrite + AsyncRead + Unpin,
    {
        pub(crate) fn into_inner(self) -> T {
            self.stream
        }
    }

    impl Connection<FailingStream, Authenticated> {
        pub(crate) fn failing() -> Self {
            Self {
                state: Default::default(),
                stream: FailingStream,
            }
        }
    }

    impl Connection<MockStream, Authenticated> {
        pub(crate) fn from_str(s: impl AsRef<str>) -> Self {
            Self {
                state: Default::default(),
                stream: MockStream::new(s.as_ref().to_owned()),
            }
        }

        pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
            Self {
                state: Default::default(),
                stream: MockStream::from_bytes(bytes),
            }
        }
    }

    #[test]
    fn test_authenticated_formats_as_debug() {
        format!("{:?}", Authenticated);
    }

    #[test]
    fn test_unauthenticated_formats_as_debug() {
        format!("{:?}", Unauthenticated);
    }

    #[test]
    fn test_formats_as_debug() {
        format!("{:?}", Connection::failing());
    }

    #[test]
    fn test_clones() {
        let _ = Connection::from_bytes(&[]).clone();
    }

    #[tokio::test]
    async fn test_connection_sends_command_with_arguments() {
        let mut connection = Connection::from_str("test_response");

        let argument_foo = "foo";
        let argument_bar = "bar";

        let _ = connection
            .send_cmd(1)
            .await
            .unwrap()
            .send_arg(&mut argument_foo.as_bytes())
            .await
            .unwrap()
            .send_arg(&mut argument_bar.as_bytes())
            .await
            .unwrap();
        let actual_buffer = connection.into_inner().to_string();
        let expected_buffer = "\u{1}foo\u{0}bar\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_connection_fails_to_send_command_with_failing_stream() {
        let mut connection = Connection::failing();
        let result = connection.send_cmd(1).await;

        let actual_error = result.err().expect("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_cloning_points_to_same_stream() {
        let connection = Connection::from_str("test_response");

        let mut cloned_connection = connection.clone();
        let _ = cloned_connection
            .send_arg(&mut "bar".as_bytes())
            .await
            .unwrap()
            .skip_arg()
            .await
            .unwrap();

        let actual_buffer = connection.into_inner().to_string();
        let actual_cloned_buffer = cloned_connection.into_inner().to_string();

        assert_eq!(actual_buffer, actual_cloned_buffer);
    }

    #[tokio::test]
    async fn test_connection_gets_response() {
        let expected_response = "test_response";
        let mut connection = Connection::from_str(format!("{}\0", expected_response));
        let actual_response = connection.get_response().await.unwrap();

        assert_eq!(expected_response, actual_response);
    }

    #[tokio::test]
    async fn test_connection_gets_response_on_failed_command() {
        let mut connection = Connection::from_str("test_error\0\u{1}");
        let actual_error = connection.get_response().await.expect_err("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::CommandFailed(message) if message == "test_error"));
    }

    #[tokio::test]
    async fn test_connection_fails_to_get_response_with_failing_stream() {
        let mut connection = Connection::failing();
        let actual_error = connection.get_response().await.expect_err("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_connection_fails_to_get_response_with_malformed_utf_8_string() {
        let non_utf8_sequence = &[0xa0 as u8, 0xa1];
        let mut connection = Connection::from_bytes(non_utf8_sequence);
        let actual_error = connection.get_response().await.expect_err("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Utf8Parse(_)));
    }

    #[tokio::test]
    async fn test_authentication_succeeds_with_correct_auth_string() {
        let expected_auth_string = "admin\0af13b20af0e0b0e3517a406c42622d3d\0";
        let stream = MockStream::new("BaseX:19501915960728\0".to_owned());
        let connection = Connection::new(stream).authenticate("admin", "admin").await.unwrap();

        let actual_auth_string = connection.into_inner().to_string();

        assert_eq!(expected_auth_string, actual_auth_string);
    }

    #[tokio::test]
    async fn test_authentication_fails_on_error_response() {
        let stream = MockStream::new("BaseX:19501915960728\0\u{1}".to_owned());
        let connection = Connection::new(stream);

        let actual_error = connection
            .authenticate("admin", "admin")
            .await
            .err()
            .expect("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Auth));
    }

    #[tokio::test]
    async fn test_read_string_from_connection() {
        let stream = MockStream::new("test_string".to_owned());
        let connection = Connection::new(stream);

        let mut actual_string = String::new();
        let _ = connection
            .into_inner()
            .read_to_string(&mut actual_string)
            .await
            .unwrap();
        let expected_string = "test_string\u{0}";

        assert_eq!(expected_string, &actual_string);
    }

    #[tokio::test]
    async fn test_read_byte_into_empty_buffer_from_connection() {
        let expected_bytes: Vec<u8> = vec![];
        let stream = MockStream::new("test_string".to_owned());
        let connection = Connection::new(stream);

        let mut actual_bytes = vec![];
        let _ = connection.into_inner().read(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[tokio::test]
    async fn test_read_single_byte_from_connection() {
        let expected_bytes = "t".as_bytes();
        let stream = MockStream::new("test_string".to_owned());
        let connection = Connection::new(stream);

        let mut actual_bytes = vec![0];
        let _ = connection.into_inner().read(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }
}
