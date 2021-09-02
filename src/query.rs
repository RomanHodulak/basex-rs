use crate::{Result, Connection, DatabaseStream};

/// Represents database command code in the [query mode](https://docs.basex.org/wiki/Query_Mode).
#[derive(Debug)]
pub enum Command {
    Close = 2,
    Bind = 3,
    Execute = 5,
    Info = 6,
    Options = 7,
    Context = 0x0e,
    Updating = 0x1e,
}

pub struct Query<T> where T: DatabaseStream {
    id: String,
    connection: Connection<T>,
}

impl<T> Query<T> where T: DatabaseStream {

    pub fn new(id: String, connection: Connection<T>) -> Self {
        Self { id, connection }
    }

    pub fn close(&mut self) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Close as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn bind(&mut self, name: &str, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Bind as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.send_arg(Some(name.as_bytes()))?;
        self.connection.send_arg(value.map(|v| v.as_bytes()))?;
        self.connection.send_arg(value_type.map(|v| v.as_bytes()))?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn execute(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Execute as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.get_response()
    }

    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Info as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.get_response()
    }

    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Options as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.get_response()
    }

    pub fn context(&mut self, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Context as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.send_arg(value.map(|v| v.as_bytes()))?;
        self.connection.send_arg(value_type.map(|v| v.as_bytes()))?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn updating(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Updating as u8)?;
        self.connection.send_arg(Some(self.id.as_bytes()))?;
        self.connection.get_response()
    }

    pub fn into_inner(self) -> Connection<T> {
        self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::MockStream;

    #[test]
    fn test_query_binds_arguments() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());

        let stream = {
            let connection = Connection::new(stream);

            let mut query = Query::new("test".to_owned(), connection);
            let _ = query.bind("foo", Some("aaa"), Some("integer"));

            query.into_inner().into_inner()
        };

        let actual_buffer = stream.to_string();
        let expected_buffer = String::from_utf8(vec![Command::Bind as u8]).unwrap()
            + "test\u{0}foo\u{0}aaa\u{0}integer\u{0}";

        assert_eq!(expected_buffer, actual_buffer);
    }
}
