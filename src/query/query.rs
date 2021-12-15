use crate::connection::Authenticated;
use crate::query::analysis::{Info, RawInfo};
use crate::query::argument::{ArgumentWriter, ToQueryArgument};
use crate::query::response::Response;
use crate::query::serializer::Options;
use crate::resource::AsResource;
use crate::{Client, Connection, DatabaseStream, Result};
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::str::FromStr;

/// Query that has its compiler [`info`] collected.
///
/// [`info`]: self::Query::info
#[derive(Debug)]
pub struct WithInfo;

/// Query that has no compiler [`info`] collected. Improves query performance due to the reduced overhead.
///
/// [`info`]: self::Query::info
#[derive(Debug)]
pub struct WithoutInfo;

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

/// Encapsulates a query argument with optional value. To bind the argument, either call [`with_input`] or
/// [`without_input`].
///
/// [`with_input`]: self::CommandWithOptionalInput::with_input
/// [`without_input`]: self::CommandWithOptionalInput::without_input
pub struct ArgumentWithOptionalValue<'a, T, HasInfo>
where
    T: DatabaseStream,
{
    query: &'a mut Query<T, HasInfo>,
}

impl<'a, T, HasInfo> ArgumentWithOptionalValue<'a, T, HasInfo>
where
    T: DatabaseStream,
{
    pub(crate) fn new(query: &'a mut Query<T, HasInfo>) -> Self {
        Self { query }
    }

    /// Sends the value to the argument, returning back the mutable reference to [`Query`].
    ///
    /// [`Query`]: self::Query
    pub fn with_value<'b, A: ToQueryArgument<'b>>(self, value: A) -> Result<&'a mut Query<T, HasInfo>> {
        value.write_xquery(&mut ArgumentWriter(self.query.connection()))?;
        self.query.connection().send_arg(&mut A::xquery_type().as_bytes())?;
        self.query.connection().get_response()?;
        Ok(self.query)
    }

    /// Omits the value from the argument, returning back the mutable reference to [`Query`].
    ///
    /// [`Query`]: self::Query
    pub fn without_value(self) -> Result<&'a mut Query<T, HasInfo>> {
        self.query.connection().skip_arg()?;
        self.query.connection().skip_arg()?;
        self.query.connection().get_response()?;
        Ok(self.query)
    }
}

/// Server query is composed of [XQuery](https://docs.basex.org/wiki/XQuery) code, which is immutable once created.
///
/// The client may [`bind`] arguments, set [`context`] or modify [`options`] which influences the way result is
/// generated.
///
/// Furthermore, the client can [`execute`] the query, check for [`updating`] statements or read compiler [`info`].
///
/// [`bind`]: self::Query::bind
/// [`context`]: self::Query::context
/// [`execute`]: self::Query::execute
/// [`info`]: self::Query::info
/// [`options`]: self::Query::options
/// [`updating`]: self::Query::updating
#[derive(Debug)]
pub struct Query<T, HasInfo = WithoutInfo>
where
    T: DatabaseStream,
{
    has_info: PhantomData<HasInfo>,
    id: String,
    client: Client<T>,
}

impl<T, HasInfo> Query<T, HasInfo>
where
    T: DatabaseStream,
{
    /// Deletes the query.
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Query, DatabaseStream, Result};
    /// # fn example<T: DatabaseStream, HasInfo>(mut query: Query<T, HasInfo>) -> Result<()> {
    /// // Delete the query, moves back the `client`.
    /// let client = query.close()?;
    /// // Current function now owns `client`.
    /// # Ok(())
    /// # }
    /// ```
    pub fn close(mut self) -> Result<Client<T>> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Close as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;
        connection.get_response()?;
        Ok(self.client)
    }

    /// Binds a variable under the given valid XML `name`.
    ///
    /// You then need to make a statement about its value using either [`with_value`] or [`without_value`].
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Client, ClientError};
    /// # use std::io::Read;
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut query = client.query("/")?.without_info()?;
    /// query
    ///     .bind("boy_sminem")?.with_value(123)?
    ///     .bind("bogdanoff")?.without_value()?;
    /// let mut response = query.execute()?;
    /// let mut result = String::new();
    /// response.read_to_string(&mut result)?;
    ///
    /// println!("{}", result);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`with_value`]: self::ArgumentWithOptionalValue::with_value
    /// [`without_value`]: self::ArgumentWithOptionalValue::without_value
    pub fn bind(&mut self, name: &str) -> Result<ArgumentWithOptionalValue<T, HasInfo>> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Bind as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;
        connection.send_arg(&mut name.as_bytes())?;
        Ok(ArgumentWithOptionalValue::new(self))
    }

    /// Executes the query and returns its response.
    ///
    /// The response is readable using the [`Read`] trait.
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Client, ClientError};
    /// # use std::io::Read;
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let query = client.query("declare variable $points := 30;
    /// <polygon>
    ///   {
    ///     for $i in 1 to $points
    ///     let $angle := 2 * math:pi() * number($i div $points)
    ///     return <point x=\"{round(math:cos($angle), 8)}\" y=\"{round(math:sin($angle), 8)}\"></point>
    ///   }
    /// </polygon>")?.without_info()?;
    ///
    /// let mut result = String::new();
    /// let mut response = query.execute()?;
    /// response.read_to_string(&mut result)?;
    ///
    /// println!("{}", result);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Read`]: std::io::Read
    pub fn execute(mut self) -> Result<Response<T, HasInfo>> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Execute as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;
        Ok(Response::new(self))
    }

    /// Returns all query serialization options.
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Client, ClientError, serializer::BooleanAttribute};
    /// # use std::io::Read;
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut query = client.query("/")?.without_info()?;
    /// let mut options = query.options()?;
    /// let client = query.close()?;
    /// options.insert("indent", BooleanAttribute::no());
    /// options.save(client)?;
    /// # Ok(())
    /// # }
    pub fn options(&mut self) -> Result<Options> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Options as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;
        let response = self.connection().get_response()?;
        Ok(Options::from_str(&response).unwrap())
    }

    /// Replaces whatever context is set (if any) to the given `value`.
    ///
    /// By default the context is set to currently opened database (if any). Setting context allows you to run query
    /// on a different data-set or without a database.
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Client, ClientError};
    /// # use std::io::Read;
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut response = {
    ///     let mut query = client.query("count(prdel/*)")?.without_info()?;
    ///     query.context("<prdel><one/><two/><three/></prdel>")?;
    ///     query.execute()?
    /// };
    /// let mut actual_result = String::new();
    /// response.read_to_string(&mut actual_result)?;
    /// response.close()?.close()?;
    ///
    /// let expected_result = "3";
    /// assert_eq!(expected_result, actual_result);
    /// # Ok(())
    /// # }
    /// ```
    pub fn context<'a>(&mut self, value: impl AsResource<'a>) -> Result<&mut Self> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Context as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;
        connection.send_arg(&mut value.into_read())?;
        connection.send_arg(&mut "document-node()".as_bytes())?;
        connection.get_response()?;
        Ok(self)
    }

    /// Checks if the query contains updating expressions.
    ///
    /// # Panics
    /// Panics when the response contains non-boolean value.
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Client, ClientError};
    /// # fn main() -> Result<(), ClientError> {
    /// # let client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///
    /// let mut query = client.query("replace value of node /None with 1")?.without_info()?;
    /// assert!(query.updating()?);
    /// # let client = query.close()?;
    ///
    /// let mut query = client.query("count(/None/*)")?.without_info()?;
    /// assert!(!query.updating()?);
    /// # query.close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn updating(&mut self) -> Result<bool> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Updating as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;

        match self.connection().get_response()?.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => panic!("Expected boolean string, got \"{}\"", other),
        }
    }

    fn connection(&mut self) -> &mut Connection<T, Authenticated> {
        self.client.borrow_mut()
    }
}

impl<T> Query<T, WithoutInfo>
where
    T: DatabaseStream,
{
    /// Attaches [`Query`] to an existing query in the session.
    ///
    /// [`Query`]: self::Query
    pub(crate) fn without_info(id: String, client: Client<T>) -> Query<T, WithoutInfo> {
        Self {
            has_info: Default::default(),
            id,
            client,
        }
    }
}

impl<T> Query<T, WithInfo>
where
    T: DatabaseStream,
{
    /// Attaches [`Query`] to an existing query in the session.
    ///
    /// [`Query`]: self::Query
    pub(crate) fn with_info(id: String, client: Client<T>) -> Query<T, WithInfo> {
        Self {
            has_info: Default::default(),
            id,
            client,
        }
    }

    /// Returns the query compilation and profiling [`Info`].
    ///
    /// # Example
    ///
    /// ```
    /// # use basex::{Query, DatabaseStream, WithInfo, analysis::Info, Result};
    /// # fn example<T: DatabaseStream>(mut query: Query<T, WithInfo>) -> Result<()> {
    /// let info = query.info()?;
    /// println!(
    ///     "Compilation took {} ms, query: {}",
    ///     info.compiling_time().as_millis(),
    ///     info.optimized_query()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Info`]: super::analysis::Info
    pub fn info(&mut self) -> Result<impl Info> {
        let connection: &mut Connection<T, Authenticated> = self.client.borrow_mut();
        connection.send_cmd(Command::Info as u8)?;
        connection.send_arg(&mut self.id.as_bytes())?;
        Ok(RawInfo::new(self.connection().get_response()?))
    }
}

impl<T, HasInfo> Borrow<Client<T>> for Query<T, HasInfo>
where
    T: DatabaseStream,
{
    fn borrow(&self) -> &Client<T> {
        &self.client
    }
}

impl<T, HasInfo> BorrowMut<Client<T>> for Query<T, HasInfo>
where
    T: DatabaseStream,
{
    fn borrow_mut(&mut self) -> &mut Client<T> {
        &mut self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::analysis::tests::QUERY_INFO;
    use crate::tests::FailingStream;
    use crate::{assert_query_info, ClientError};
    use std::io::{empty, Read};

    impl<T, HasInfo> Query<T, HasInfo>
    where
        T: DatabaseStream,
    {
        pub(crate) fn into_inner(self) -> Connection<T, Authenticated> {
            self.client.into_inner()
        }
    }

    #[test]
    fn test_with_info_formats_as_debug() {
        format!("{:?}", WithInfo);
    }

    #[test]
    fn test_without_info_formats_as_debug() {
        format!("{:?}", WithoutInfo);
    }

    #[test]
    fn test_formats_as_debug() {
        format!(
            "{:?}",
            Query::with_info("".to_owned(), Client::new(Connection::failing()))
        );
    }

    #[test]
    fn test_borrows_as_client() {
        let _: &Client<FailingStream> = Query::with_info("".to_owned(), Client::new(Connection::failing())).borrow();
    }

    #[test]
    fn test_query_binds_arguments() -> Result<()> {
        let connection = Connection::from_str("\0\0\0\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));

        query
            .bind("foo")?
            .with_value("aaa")?
            .bind("bar")?
            .with_value(123)?
            .bind("void")?
            .without_value()?;

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{3}test\u{0}foo\u{0}aaa\u{0}xs:string\u{0}\
            \u{3}test\u{0}bar\u{0}123\u{0}xs:int\u{0}\
            \u{3}test\u{0}void\u{0}\u{0}\u{0}"
            .to_owned();

        assert_eq!(expected_buffer, actual_buffer);
        Ok(())
    }

    #[test]
    fn test_query_fails_to_bind_argument_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.bind("foo").err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_binds_value_to_context() {
        let connection = Connection::from_str("\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let _ = query.context("aaa").unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}aaa\u{0}document-node()\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_binds_value_to_context_with_empty_type() {
        let connection = Connection::from_str("\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let _ = query.context("aaa").unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}aaa\u{0}document-node()\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_binds_empty_value_to_context() {
        let connection = Connection::from_str("\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let _ = query.context(&mut empty()).unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}\u{0}document-node()\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_bind_context_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.context(&mut empty()).err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_executes() {
        let expected_response = "test_response";
        let connection = Connection::from_str(expected_response.to_owned() + "\0");

        let query = Query::with_info("test".to_owned(), Client::new(connection));
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
        let connection = Connection::failing();

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.execute().err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_updating_command() {
        let connection = Connection::from_str("true\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.updating().unwrap();

        assert!(actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{1e}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_runs_non_updating_command() {
        let connection = Connection::from_str("false\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
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
        let connection = Connection::from_str("test_response\0");
        let client = Client::new(connection);

        let mut query = Query::with_info("test".to_owned(), client);
        let _ = query.updating().unwrap();
    }

    #[test]
    fn test_query_fails_to_run_updating_command_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.updating().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_options_command() {
        let expected_response = "ident=no";
        let connection = Connection::from_str(&format!("{}\0\0", expected_response));

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.options().unwrap();

        assert_eq!(expected_response, &actual_response.to_string());

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{7}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_run_options_command_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.options().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_runs_info_command() {
        let expected_response = QUERY_INFO;
        let connection = Connection::from_str(&format!("{}\0\0", expected_response));

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.info().unwrap();

        assert_query_info!(actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{6}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_run_info_command_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.info().expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_query_closes() {
        let expected_response = "test_response\0";
        let connection = Connection::from_str(expected_response);

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let client = query.close().unwrap();

        let stream = client.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{2}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[test]
    fn test_query_fails_to_close_with_failing_stream() {
        let connection = Connection::failing();

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.close().err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }
}
