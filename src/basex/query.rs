use super::{Result, Connection};
use std::io::{Read, Write};
use crate::basex::DatabaseStream;

/// Represents database command code in the [query mode](https://docs.basex.org/wiki/Query_Mode).
#[derive(Debug)]
pub enum Command {
    Close = 2,
    Bind = 3,
    Results = 4,
    Execute = 5,
    Info = 6,
    Options = 7,
    Context = 0x0e,
    Updating = 0x1e,
}

pub struct Query<'a, T> where T: DatabaseStream<'a> {
    id: String,
    connection: Connection<'a, T>,
}

impl<'a, T> Query<'a, T> where T: DatabaseStream<'a> {

    pub fn new(id: String, connection: Connection<'a, T>) -> Self {
        Self { id, connection }
    }

    pub fn close(&mut self) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Close as u8, vec![Some(&self.id)])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn bind(&mut self, name: &str, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Bind as u8, vec![Some(&self.id), Some(name), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn execute(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Execute as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Info as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Options as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn context(&mut self, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Context as u8, vec![Some(&self.id), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn updating(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Updating as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn into_inner(self) -> Connection<'a, T> {
        self.connection
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

    #[test]
    fn test_query_binds_arguments() {
        let mut buffer = vec![];
        let expected_response = "test_response";
        let stream = MockStream::new(&mut buffer, expected_response.to_owned());

        let stream = {
            let connection = Connection::new(stream);

            let mut query = Query::new("test".to_owned(), connection);
            query.bind("foo", Some("aaa"), Some("integer"));

            query.into_inner().into_inner()
        };

        let actual_buffer = stream.to_string();
        let expected_buffer = String::from_utf8(vec![Command::Bind as u8]).unwrap()
            + "test\u{0}foo\u{0}aaa\u{0}integer\u{0}";

        assert_eq!(expected_buffer, actual_buffer);
    }
}
