use crate::connection::escape_reader::EscapeReader;
use crate::{ClientError, Result, Stream};
use std::io::{copy, Read};
use std::marker::PhantomData;

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
pub struct Connection<T, State = Unauthenticated>
where
    T: Stream,
{
    state: PhantomData<State>,
    stream: T,
}

impl<T> Connection<T, Unauthenticated>
where
    T: Stream,
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
    pub fn authenticate(mut self, user: &str, password: &str) -> Result<Connection<T, Authenticated>> {
        let response = self.read_string()?;

        let challenge: Vec<&str> = response.split(':').collect();
        let server_name = challenge[0];
        let timestamp = challenge[1];

        let first_digest = md5::compute(&format!("{}:{}:{}", user, server_name, password));
        let second_digest = md5::compute(&format!("{:x}{}", first_digest, timestamp));

        let auth_string = format!("{}\0{:x}\0", user, second_digest);
        let mut control_byte: [u8; 1] = [0];

        self.stream.write_all(auth_string.as_bytes())?;
        self.stream.read_exact(&mut control_byte)?;

        if control_byte[0] != 0 {
            return Err(ClientError::Auth);
        }

        Ok(Connection {
            state: Default::default(),
            stream: self.stream,
        })
    }
}

impl<T> Connection<T, Authenticated>
where
    T: Stream,
{
    pub(crate) fn send_cmd(&mut self, code: u8) -> Result<&mut Self> {
        self.stream.write_all(&[code])?;

        Ok(self)
    }

    pub(crate) fn send_arg(&mut self, argument: &mut impl Read) -> Result<&mut Self> {
        copy(&mut EscapeReader::new(argument), &mut self.stream)?;

        self.skip_arg()
    }

    pub(crate) fn skip_arg(&mut self) -> Result<&mut Self> {
        self.stream.write_all(&[0])?;

        Ok(self)
    }

    /// Gets response string, and returns string if command was successful. Returns `CommandFailed`
    /// error with a message otherwise.
    pub(crate) fn get_response(&mut self) -> Result<String> {
        let info = self.read_string()?;

        if self.is_ok()? {
            Ok(info)
        } else {
            Err(ClientError::CommandFailed(info))
        }
    }

    /// Reads return code and decodes it to TRUE on success or FALSE on error.
    pub(crate) fn is_ok(&mut self) -> Result<bool> {
        let mut buf: [u8; 1] = [0];
        self.stream.read_exact(&mut buf)?;

        Ok(buf[0] == 0)
    }
}

impl<T, State> Read for Connection<T, State>
where
    T: Stream,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match buf.is_empty() {
            true => Ok(0),
            false => self.stream.read(buf),
        }
    }
}

impl<T, State> Connection<T, State>
where
    T: Stream,
{
    /// Creates a new connection with a new independently owned handle to the underlying socket.
    pub(crate) fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            state: Default::default(),
            stream: self.stream.try_clone()?,
        })
    }

    pub(crate) fn read_string(&mut self) -> Result<String> {
        let mut raw_string: Vec<u8> = vec![];
        loop {
            let mut buf: [u8; 1] = [0];
            self.stream.read_exact(&mut buf)?;

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
    T: Stream,
{
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

/// Contains [`Connection`]. Call [`HasConnection::connection`] to get mutable handle to use [`Connection`].
///
/// [`Connection`]: self::Connection
/// [`HasConnection::connection`]: self::HasConnection::connection
pub(crate) trait HasConnection<T: Stream> {
    /// Returns mutable reference to wrapped [`Connection`]. This is useful to make low-level calls to the server
    /// protocol but can result in unexpected state changes when used improperly.
    ///
    /// [`Connection`]: self::Connection
    fn connection(&mut self) -> &mut Connection<T, Authenticated>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{FailingStream, MockStream};
    use std::io::Read;

    impl<T, State> Connection<T, State>
    where
        T: Stream,
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

    #[test]
    fn test_connection_sends_command_with_arguments() {
        let mut connection = Connection::from_str("test_response");

        let argument_foo = "foo";
        let argument_bar = "bar";

        let _ = connection
            .send_cmd(1)
            .unwrap()
            .send_arg(&mut argument_foo.as_bytes())
            .unwrap()
            .send_arg(&mut argument_bar.as_bytes())
            .unwrap();
        let actual_buffer = connection.into_inner().to_string();
        let expected_buffer = "\u{1}foo\u{0}bar\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_connection_fails_to_send_command_with_failing_stream() {
        let mut connection = Connection::failing();
        let result = connection.send_cmd(1);

        let actual_error = result.err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_cloning_points_to_same_stream() {
        let connection = Connection::from_str("test_response");

        let mut cloned_connection = connection.try_clone().unwrap();
        let _ = cloned_connection
            .send_arg(&mut "bar".as_bytes())
            .unwrap()
            .skip_arg()
            .unwrap();

        let actual_buffer = connection.into_inner().to_string();
        let actual_cloned_buffer = cloned_connection.into_inner().to_string();

        assert_eq!(actual_buffer, actual_cloned_buffer);
    }

    #[test]
    fn test_connection_gets_response() {
        let expected_response = "test_response";
        let mut connection = Connection::from_str(format!("{}\0", expected_response));
        let actual_response = connection.get_response().unwrap();

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_connection_gets_response_on_failed_command() {
        let mut connection = Connection::from_str("test_error\0\u{1}");
        let actual_error = connection.get_response().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::CommandFailed(message) if message == "test_error"));
    }

    #[test]
    fn test_connection_fails_to_get_response_with_failing_stream() {
        let mut connection = Connection::failing();
        let actual_error = connection.get_response().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_connection_fails_to_get_response_with_malformed_utf_8_string() {
        let non_utf8_sequence = &[0xa0 as u8, 0xa1];
        let mut connection = Connection::from_bytes(non_utf8_sequence);
        let actual_error = connection.get_response().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Utf8Parse(_)));
    }

    #[test]
    fn test_authentication_succeeds_with_correct_auth_string() {
        let expected_auth_string = "admin\0af13b20af0e0b0e3517a406c42622d3d\0";
        let stream = MockStream::new("BaseX:19501915960728\0".to_owned());
        let connection = Connection::new(stream).authenticate("admin", "admin").unwrap();

        let actual_auth_string = connection.into_inner().to_string();

        assert_eq!(expected_auth_string, actual_auth_string);
    }

    #[test]
    fn test_authentication_fails_on_error_response() {
        let stream = MockStream::new("BaseX:19501915960728\0\u{1}".to_owned());
        let connection = Connection::new(stream);

        let actual_error = connection
            .authenticate("admin", "admin")
            .err()
            .expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Auth));
    }

    #[test]
    fn test_read_string_from_connection() {
        let stream = MockStream::new("test_string".to_owned());
        let mut connection = Connection::new(stream);

        let mut actual_string = String::new();
        let _ = connection.read_to_string(&mut actual_string).unwrap();
        let expected_string = "test_string\u{0}";

        assert_eq!(expected_string, &actual_string);
    }

    #[test]
    fn test_read_byte_into_empty_buffer_from_connection() {
        let expected_bytes: Vec<u8> = vec![];
        let stream = MockStream::new("test_string".to_owned());
        let mut connection = Connection::new(stream);

        let mut actual_bytes = vec![];
        let _ = connection.read(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[test]
    fn test_read_single_byte_from_connection() {
        let expected_bytes = "t".as_bytes();
        let stream = MockStream::new("test_string".to_owned());
        let mut connection = Connection::new(stream);

        let mut actual_bytes = vec![0];
        let _ = connection.read(&mut actual_bytes).unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }
}
