use crate::{Result, Connection, DatabaseStream, Client};
use crate::query::response::Response;

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

/// Represents an [XQuery](https://docs.basex.org/wiki/XQuery) code uniquely identified by the database.
///
/// Database query is created out of an XQuery syntax string. The XQuery gets send to the database and associated with
/// an ID. Once the query ID is assigned, the XQuery cannot be changed, but the client may bind arguments or context
/// for it.
///
/// Once happy with the arguments bound or context set, the Query can be executed or analysed.
pub struct Query<T> where T: DatabaseStream {
    id: String,
    connection: Connection<T>,
}

impl<T> Query<T> where T: DatabaseStream {

    pub(crate) fn new(id: String, connection: Connection<T>) -> Self {
        Self { id, connection }
    }

    /// Closes and unregisters the query with the specified id.
    pub fn close(mut self) -> Result<Client<T>> {
        self.connection.send_cmd(Command::Close as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()?;
        Ok(Client::new(self.connection))
    }

    /// Binds a value to a variable. The type will be ignored if the value is `None`.
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

    /// Executes the query and returns its response.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    /// use std::io::Read;
    ///
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut xquery = "declare variable $points := 30;
    /// <polygon>
    ///   {
    ///     for $i in 1 to $points
    ///     let $angle := 2 * math:pi() * number($i div $points)
    ///     return <point x=\"{round(math:cos($angle), 8)}\" y=\"{round(math:sin($angle), 8)}\"></point>
    ///   }
    /// </polygon>".as_bytes();
    /// let query = client.query(&mut xquery)?;
    ///
    /// let mut result = String::new();
    /// let mut response = query.execute()?;
    /// response.read_to_string(&mut result)?;
    ///
    /// println!("{}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute(mut self) -> Result<Response<T>> {
        self.connection.send_cmd(Command::Execute as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        Ok(Response::new(self.id, self.connection))
    }

    /// Returns a string with query compilation and profiling info.
    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Info as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()
    }

    /// Returns a string with all query serialization parameters, which can e.g. be assigned to the SERIALIZER option.
    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Options as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;
        self.connection.get_response()
    }

    /// Binds a value to the context. The type will be ignored if the value is `None`.
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

    /// Checks if the query contains updating expressions.
    pub fn updating(&mut self) -> Result<bool> {
        self.connection.send_cmd(Command::Updating as u8)?;
        self.connection.send_arg(&mut self.id.as_bytes())?;

        match self.connection.get_response()?.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => panic!("Expected boolean string, got \"{}\"", other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{MockStream, FailingStream};
    use crate::ClientError;
    use std::io::Read;

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
        let stream = MockStream::new(expected_response.to_owned() + "\0");
        let connection = Connection::new(stream);

        let query = Query::new("test".to_owned(), connection);
        let mut actual_response = String::new();
        let mut response = query.execute().unwrap();
        response.read_to_string(&mut actual_response).unwrap();

        assert_eq!(expected_response, actual_response);

        let query = response.close().unwrap();
        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{5}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_execute_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let query = Query::new("test".to_owned(), connection);
        let actual_error = query.execute().err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_updating_command() {
        let stream = MockStream::new("true".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_response = query.updating().unwrap();

        assert!(actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{1e}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_runs_non_updating_command() {
        let stream = MockStream::new("false".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let actual_response = query.updating().unwrap();

        assert!(!actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{1e}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    #[should_panic]
    fn test_query_panics_updating_command_response_is_not_bool() {
        let stream = MockStream::new("test_response".to_owned());
        let connection = Connection::new(stream);

        let mut query = Query::new("test".to_owned(), connection);
        let _ = query.updating().unwrap();
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

        let query = Query::new("test".to_owned(), connection);
        let client = query.close().unwrap();

        let stream = client.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{2}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_close_with_failing_stream() {
        let connection = Connection::new(FailingStream);

        let query = Query::new("test".to_owned(), connection);
        let actual_error = query.close().err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }
}
