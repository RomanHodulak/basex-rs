use crate::{ClientError, DatabaseStream, Result};
use std::io::{ Read, copy };

/// Responsible for low-level communication with the stream. It does not understand what commands can be run, as
/// opposed to the [`Client`] or what [`Query`] can do, but can serialize commands and process responses.
pub struct Connection<T> where T: DatabaseStream {
    stream: T,
}

impl<T> Connection<T> where T: DatabaseStream {

    /// Creates a connection that communicates with the database via the provided `stream`.
    pub fn new(stream: T) -> Self {
        Self { stream }
    }

    /// Authenticates the connection using the
    /// [server protocol](https://docs.basex.org/wiki/Server_Protocol#Authentication). Being authenticated is the
    /// pre-requisite for every other method to work.
    ///
    /// # Arguments
    /// * `user`: Username.
    /// * `password`: Password.
    pub fn authenticate(&mut self, user: &str, password: &str) -> Result<&Self> {
        let response = self.read_string()?;

        let challenge: Vec<&str> = response.split(":").collect();
        let server_name = challenge[0];
        let timestamp = challenge[1];

        let first_digest = md5::compute(&format!("{}:{}:{}", user, server_name, password));
        let second_digest = md5::compute(&format!("{:x}{}", first_digest, timestamp));

        let auth_string = format!("{}\0{:x}\0", user, second_digest);
        let mut control_byte: [u8; 1] = [0];

        self.stream.write(auth_string.as_bytes())?;
        self.stream.read(&mut control_byte)?;

        if control_byte[0] != 0 {
            return Err(ClientError::Auth);
        }

        Ok(self)
    }

    fn read_string(&mut self) -> Result<String> {
        let mut raw_string: Vec<u8> = vec![];
        loop {
            let mut buf: [u8; 1] = [0];
            self.stream.read(&mut buf)?;

            if buf[0] == 0 {
                break;
            }
            raw_string.push(buf[0]);
        }

        Ok(String::from_utf8(raw_string)?)
    }

    pub(crate) fn send_cmd(&mut self, code: u8) -> Result<&mut Self> {
        self.stream.write(&[code])?;

        Ok(self)
    }

    pub(crate) fn send_arg<R: Read>(&mut self, argument: &mut R) -> Result<&mut Self> {
        copy(argument, &mut self.stream)?;

        self.skip_arg()
    }

    pub(crate) fn skip_arg(&mut self) -> Result<&mut Self> {
        self.stream.write(&[0])?;

        Ok(self)
    }

    /// Gets response string, and returns string if command was successful. Returns `CommandFailed`
    /// error with a message otherwise.
    pub(crate) fn get_response(&mut self) -> Result<String> {
        let info = self.read_string()?;

        if self.is_ok()? {
            Ok(info)
        }
        else {
            Err(ClientError::CommandFailed { message: info })
        }
    }

    /// Reads return code and decodes it to TRUE on success or FALSE on error.
    fn is_ok(&mut self) -> Result<bool> {
        let mut buf: [u8; 1] = [0];
        self.stream.read(&mut buf)?;

        Ok(buf[0] == 0)
    }

    /// Creates a new connection with a new independently owned handle to the underlying socket.
    pub(crate) fn try_clone(&mut self) -> Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?,
        })
    }
}

impl<T> Read for Connection<T> where T: DatabaseStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let len = self.stream.read(buf)?;
        let last_byte = buf.last().unwrap();

        if *last_byte == 0 {
            return Ok(if len > 0 { len - 1 } else { 0 });
        }

        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{MockStream, FailingStream};
    use std::io::Read;

    impl<T> Connection<T> where T: DatabaseStream {
        pub(crate) fn into_inner(self) -> T {
            self.stream
        }
    }

    #[test]
    fn test_connection_sends_command_with_arguments() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let mut connection = Connection::new(stream);

        let argument_foo = "foo";
        let argument_bar = "bar";

        let _ = connection.send_cmd(1).unwrap()
            .send_arg(&mut argument_foo.as_bytes()).unwrap()
            .send_arg(&mut argument_bar.as_bytes()).unwrap();
        let actual_buffer = connection.into_inner().to_string();
        let expected_buffer = "\u{1}foo\u{0}bar\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_connection_fails_to_send_command_with_failing_stream() {
        let mut connection = Connection::new(FailingStream);
        let result = connection.send_cmd(1);

        let actual_error = result.err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_cloning_points_to_same_stream() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let mut connection = Connection::new(stream);

        let mut cloned_connection = connection.try_clone().unwrap();
        let _ = cloned_connection.send_arg(&mut "bar".as_bytes()).unwrap()
            .skip_arg().unwrap();

        let actual_buffer = connection.into_inner().to_string();
        let actual_cloned_buffer = cloned_connection.into_inner().to_string();

        assert_eq!(actual_buffer, actual_cloned_buffer);
    }

    #[test]
    fn test_connection_gets_response() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let mut connection = Connection::new(stream);
        let actual_response = connection.get_response().unwrap();

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_connection_gets_response_on_failed_command() {
        let expected_response = "test_error\0\u{1}";
        let stream = MockStream::new(expected_response.to_owned());
        let mut connection = Connection::new(stream);
        let actual_error = connection.get_response().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::CommandFailed{ message } if message == "test_error"));
    }

    #[test]
    fn test_connection_fails_to_get_response_with_failing_stream() {
        let mut connection = Connection::new(FailingStream);
        let actual_error = connection.get_response().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_connection_fails_to_get_response_with_malformed_utf_8_string() {
        let non_utf8_sequence = &[0xa0 as u8, 0xa1];
        let stream = MockStream::from_bytes(non_utf8_sequence);
        let mut connection = Connection::new(stream);
        let actual_error = connection.get_response().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Utf8Parse(_)));
    }

    #[test]
    fn test_authentication_succeeds_with_correct_auth_string() {
        let expected_auth_string = "admin\0af13b20af0e0b0e3517a406c42622d3d\0";
        let stream = MockStream::new("BaseX:19501915960728\0".to_owned());
        let mut connection = Connection::new(stream);

        let _ = connection.authenticate("admin", "admin").unwrap();

        let actual_auth_string = connection.into_inner().to_string();

        assert_eq!(expected_auth_string, actual_auth_string);
    }

    #[test]
    fn test_authentication_fails_on_error_response() {
        let stream = MockStream::new("BaseX:19501915960728\0\u{1}".to_owned());
        let mut connection = Connection::new(stream);

        let actual_error = connection.authenticate("admin", "admin")
            .err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Auth));
    }

    #[test]
    fn test_read_string_from_connection() {
        let expected_string = "test_string";
        let stream = MockStream::new(expected_string.to_owned());
        let mut connection = Connection::new(stream);

        let mut actual_string = String::new();
        let _ = connection.read_to_string(&mut actual_string).unwrap();

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
