use crate::basex::{ClientError, DatabaseStream};
use super::Result;
use std::io::{Write, Read};
use std::marker::PhantomData;

pub struct Connection<'a, T> where T: DatabaseStream<'a> {
    stream: T,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> Connection<'a, T> where T: DatabaseStream<'a> {

    pub fn new(stream: T) -> Self {
        Self { stream, phantom: PhantomData }
    }

    pub(crate) fn authenticate(&mut self, user: &str, password: &str) -> Result<&Self> {
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

    /// Sends command identified by the code and supplies the given arguments.
    pub(crate) fn send_cmd(&mut self, code: u8, arguments: Vec<Option<&str>>) -> Result<&Self> {
        let mut data: Vec<u8> = vec![code];

        for argument in arguments {
            if argument.is_some() {
                data.extend_from_slice(argument.unwrap().as_bytes());
            }
            data.push(0);
        }

        self.stream.write(&data)?;

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
    pub(crate) fn try_clone(&'a mut self) -> Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?,
            phantom: PhantomData
        })
    }

    pub(crate) fn into_inner(self) -> T {
        self.stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockStream<'a> {
        buffer: &'a mut Vec<u8>,
        response: String,
    }

    impl<'a> MockStream<'a> {
        fn new(buffer: &'a mut Vec<u8>, response: String) -> Self {
            Self { buffer, response }
        }
    }

    impl ToString for MockStream<'_> {
        fn to_string(&self) -> String {
            String::from_utf8(self.buffer.clone()).unwrap()
        }
    }

    impl Read for MockStream<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let size = self.response.as_bytes().len();
            (&mut *buf).write_all(self.response.as_bytes());
            (&mut *buf).write(&[0 as u8]);
            Ok(size)
        }
    }

    impl Write for MockStream<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let bytes_written = buf.len();
            self.buffer.extend(buf);
            Ok(bytes_written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            todo!()
        }
    }

    impl<'a> DatabaseStream<'a> for MockStream<'a> {
        fn try_clone(&'a mut self) -> Result<Self> {
            Ok(MockStream::new(self.buffer, self.response.clone()))
        }
    }

    impl<'a> Connection<'a, MockStream<'a>> {
        fn buffer_as_string(&'a mut self) -> String {
            self.stream.to_string()
        }
    }

    #[test]
    fn test_connection_sends_command_with_arguments() {
        let mut buffer = vec![];
        let expected_response = "test_response";
        let stream = MockStream::new(&mut buffer, expected_response.to_owned());
        let mut connection = Connection::new(stream);

        let argument_foo = "foo";
        let argument_bar = "bar";

        connection.send_cmd(1, vec![Some(argument_foo), Some(argument_bar)]);
        let actual_buffer = connection.buffer_as_string();
        let expected_buffer = "\u{1}foo\u{0}bar\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer, "Connection properly sends command with arguments {} and {}", argument_foo, argument_bar);
    }

    #[test]
    fn test_connection_fails_to_send_command_with_failing_stream() {
        struct FailingStream;

        impl Read for FailingStream {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                panic!("Unexpected call to read")
            }
        }

        impl Write for FailingStream {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(std::io::ErrorKind::Other, ""))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                unimplemented!()
            }
        }

        impl<'a> DatabaseStream<'a> for FailingStream {
            fn try_clone(&'a mut self) -> Result<Self> {
                unimplemented!()
            }
        }

        let mut connection = Connection::new(FailingStream);

        let result = connection.send_cmd(1, vec![]);
        let actual_error = result.err().expect("Operation must fail");
        let expected_error = ClientError::Io(std::io::Error::new(std::io::ErrorKind::Other, ""));

        assert!(matches!(expected_error, actual_error));
    }
}
