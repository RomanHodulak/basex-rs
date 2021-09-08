use crate::{Result, Connection, DatabaseStream};

/// Represents database command code in the [query mode](https://docs.basex.org/wiki/Query_Mode).
enum Command {
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

    pub(crate) fn new(id: String, connection: Connection<T>) -> Self {
        Self { id, connection }
    }

    pub fn close(&mut self) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Close as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn bind(&mut self, name: &str, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Bind as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.send_arg(&mut name.as_bytes())?;
        match value {
            Some(v) => self.connection.send_arg(&mut v.as_bytes())?,
            None => self.connection.skip_arg()?,
        };
        match value_type {
            Some(v) => self.connection.send_arg(&mut v.as_bytes())?,
            None => self.connection.skip_arg()?,
        };
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn execute(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Execute as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()
    }

    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Info as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()
    }

    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Options as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()
    }

    pub fn context(&mut self, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Context as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        match value {
            Some(v) => self.connection.send_arg(&mut v.as_bytes())?,
            None => self.connection.skip_arg()?,
        };
        match value_type {
            Some(v) => self.connection.send_arg(&mut v.as_bytes())?,
            None => self.connection.skip_arg()?,
        };
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn updating(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Updating as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{MockStream, FailingStream};
    use crate::ClientError;

    impl<T> Query<T> where T: DatabaseStream {
        pub(crate) fn into_inner(self) -> Connection<T> {
            self.connection
        }
    }

    #[test]
    fn test_query_binds_arguments() {
        let stream = MockStream::new("test_response".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let _ = query.bind("foo", Some("aaa"), Some("integer")).unwrap()
            .bind("bar", Some("123"), None).unwrap()
            .bind("void", None, None).unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{3}test\u{0}foo\u{0}aaa\u{0}integer\u{0}\
            \u{3}test\u{0}bar\u{0}123\u{0}\u{0}\
            \u{3}test\u{0}void\u{0}\u{0}\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_bind_argument_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.bind("foo", None, None)
            .err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_binds_value_to_context() {
        let stream = MockStream::new("test_response".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let _ = query.context(Some("aaa"), Some("integer")).unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}aaa\u{0}integer\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_binds_value_to_context_with_empty_type() {
        let stream = MockStream::new("test_response".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let _ = query.context(Some("aaa"), None).unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}aaa\u{0}\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_binds_empty_value_to_context() {
        let stream = MockStream::new("test_response".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let _ = query.context(None, None).unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}\u{0}\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_bind_context_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.context(None, None)
            .err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_executes() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_response = query.execute().unwrap();

        assert_eq!(expected_response, actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{5}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_execute_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.execute().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_updating_command() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_response = query.updating().unwrap();

        assert_eq!(expected_response, actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{1e}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_run_updating_command_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.updating().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_options_command() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_response = query.options().unwrap();

        assert_eq!(expected_response, actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{7}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_run_options_command_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.options().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_info_command() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_response = query.info().unwrap();

        assert_eq!(expected_response, actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{6}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_run_info_command_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.info().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_closes() {
        let expected_response = "test_response";
        let stream = MockStream::new(expected_response.to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let _ = query.close().unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{2}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_close_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_error = query.close().err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }
}
