use crate::asynchronous::client::Client;
use crate::asynchronous::connection::{Authenticated, Connection, ConnectionError, HasConnection};
use crate::asynchronous::query::argument::{ArgumentWriter, ToQueryArgument};
use crate::asynchronous::query::compiler::{Info, RawInfo};
use crate::asynchronous::query::response::Response;
use crate::asynchronous::query::serializer::Options;
use crate::asynchronous::resource::AsResource;
use std::marker::PhantomData;
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Result<T> = std::result::Result<T, ConnectionError>;

/// [`Query`] that has its compiler [`info`] collected. Adds some overhead to compile time.
///
/// [`Query`]: self::Query
/// [`info`]: self::Query::info
#[derive(Debug)]
pub struct WithInfo;

/// [`Query`] that has no compiler [`info`] collected. Removes some overhead from compile time.
///
/// [`Query`]: self::Query
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
/// [`with_input`]: crate::CommandWithOptionalInput::with_input
/// [`without_input`]: crate::CommandWithOptionalInput::without_input
#[derive(Debug)]
pub struct ArgumentWithOptionalValue<'a, T, HasInfo>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin,
{
    query: Pin<&'a mut Query<T, HasInfo>>,
}

impl<'a, T, HasInfo> ArgumentWithOptionalValue<'a, T, HasInfo>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send,
{
    pub(crate) fn new(query: Pin<&'a mut Query<T, HasInfo>>) -> Self {
        Self { query }
    }

    /// Sends the value to the argument, returning back the mutable reference to [`Query`].
    ///
    /// [`Query`]: self::Query
    pub async fn with_value<'b, A: ToQueryArgument<'b>>(mut self, value: A) -> Result<&'a mut Query<T, HasInfo>> {
        let mut connection = self.query.as_mut().connection();
        value.write_xquery(ArgumentWriter(&mut connection)).await?;
        connection.send_arg(&mut A::xquery_type().as_bytes()).await?;
        connection.get_response().await?;
        Ok(self.query.get_mut())
    }

    /// Omits the value from the argument, returning back the mutable reference to [`Query`].
    ///
    /// [`Query`]: self::Query
    pub async fn without_value(mut self) -> Result<&'a mut Query<T, HasInfo>> {
        let mut connection = self.query.as_mut().connection();
        connection.skip_arg().await?;
        connection.skip_arg().await?;
        connection.get_response().await?;
        Ok(self.query.get_mut())
    }
}

/// Server query is composed of [XQuery](https://docs.basex.org/wiki/XQuery) code, which is immutable once created.
///
/// You may [`bind`] arguments, set [`context`] or modify [`options`] which influences the way result is generated.
///
/// Furthermore, you can [`execute`] the query, check for [`updating`] statements or read compiler [`info`].
///
/// [`bind`]: self::Query::bind
/// [`context`]: self::Query::context
/// [`execute`]: self::Query::execute
/// [`info`]: self::Query::info
/// [`options`]: self::Query::options
/// [`updating`]: self::Query::updating
#[derive(Debug)]
#[pin_project]
pub struct Query<T, HasInfo = WithoutInfo>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin,
{
    has_info: PhantomData<HasInfo>,
    id: String,
    #[pin]
    client: Client<T>,
}

impl<T, HasInfo> Query<T, HasInfo>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send,
    HasInfo: std::marker::Unpin,
{
    /// Deletes the query.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Query, Stream, Result};
    /// # use tokio::io::{AsyncReadExt, AsyncWriteExt};
    /// # fn example<T: AsyncWriteExt + AsyncReadExt + Unpin + Stream, HasInfo>(mut query: Query<T, HasInfo>) -> Result<()> {
    /// // Delete the query, moves back the `client`.
    /// let client = query.close()?;
    /// // Current function now owns `client`.
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(mut self) -> Result<Client<T>> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(&mut self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Close as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;
        connection.get_response().await?;
        Ok(self.client)
    }

    /// Binds a variable under the given valid XML `name`.
    ///
    /// You then need to make a statement about its value using either [`with_value`] or [`without_value`].
    ///
    /// # Examples
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
    /// [`with_value`]: crate::ArgumentWithOptionalValue::with_value
    /// [`without_value`]: crate::ArgumentWithOptionalValue::without_value
    pub async fn bind(&mut self, name: &str) -> Result<ArgumentWithOptionalValue<'_, T, HasInfo>> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Bind as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;
        connection.send_arg(&mut name.as_bytes()).await?;
        Ok(ArgumentWithOptionalValue::new(pinned_self))
    }

    /// Executes the query and returns its response.
    ///
    /// The response is readable using the [`Read`] trait.
    ///
    /// # Examples
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
    pub async fn execute(mut self) -> Result<Response<T, HasInfo>> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(&mut self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Execute as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;
        Ok(Response::new(self))
    }

    /// Returns all query serialization options.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, ClientError};
    /// # use std::io::Read;
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut query = client.query("/")?.without_info()?;
    /// let mut options = query.options()?;
    /// let client = query.close()?;
    /// options.set("indent", false);
    /// options.save(client)?;
    /// # Ok(())
    /// # }
    pub async fn options(&mut self) -> Result<Options> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Options as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;
        let response = connection.get_response().await?;
        Ok(Options::from_str(&response).unwrap())
    }

    /// Replaces whatever context is set (if any) to the given `value`.
    ///
    /// By default the context is set to currently opened database (if any). Setting context allows you to run query
    /// on a different data-set or without a database.
    ///
    /// # Examples
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
    pub async fn context<'a>(&mut self, value: impl AsResource<'a>) -> Result<&mut Self> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Context as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;
        connection.send_arg(&mut value.into_read()).await?;
        connection.send_arg(&mut "document-node()".as_bytes()).await?;
        connection.get_response().await?;
        Ok(pinned_self.get_mut())
    }

    /// Checks if the query contains updating expressions.
    ///
    /// # Panics
    /// Panics when the response contains non-boolean value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::Client;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::connect("localhost", 1984, "admin", "admin")?;
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
    pub async fn updating(&mut self) -> Result<bool> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Updating as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;

        match connection.get_response().await?.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => panic!("Expected boolean string, got \"{}\"", other),
        }
    }
}
use pin_project::pin_project;
use std::pin::Pin;

impl<T: AsyncWriteExt + AsyncReadExt + Unpin, HasInfo> HasConnection<T> for Query<T, HasInfo> {
    fn connection(self: Pin<&mut Self>) -> Pin<&mut Connection<T, Authenticated>> {
        let this = self.project();

        this.client.connection()
    }
}

impl<T> Query<T, WithoutInfo>
where
    T: AsyncWriteExt + AsyncReadExt + Unpin,
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
    T: AsyncWriteExt + AsyncReadExt + Unpin,
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
    /// # Examples
    ///
    /// ```
    /// # use basex::{Query, Stream, WithInfo, compiler::Info, Result};
    /// # use tokio::io::{AsyncWriteExt, AsyncReadExt};
    /// # fn example<T: AsyncWriteExt + AsyncReadExt + Unpin + Stream>(mut query: Query<T, WithInfo>) -> Result<()> {
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
    /// [`Info`]: crate::compiler::Info
    pub async fn info(&mut self) -> Result<impl Info> {
        let id = self.id.clone();
        let mut pinned_self = Pin::new(self);
        let this = pinned_self.as_mut().project();
        let mut connection = this.client.connection();
        connection.send_cmd(Command::Info as u8).await?;
        connection.send_arg(&mut id.as_bytes()).await?;
        Ok(RawInfo::new(connection.get_response().await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_query_info;
    use crate::query::compiler::tests::QUERY_INFO;
    use tokio::io::{empty, AsyncReadExt, AsyncWriteExt};

    impl<T, HasInfo> Query<T, HasInfo>
    where
        T: AsyncWriteExt + AsyncReadExt + Unpin,
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

    #[tokio::test]
    async fn test_query_binds_arguments() -> Result<()> {
        let connection = Connection::from_str("\0\0\0\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));

        query
            .bind("foo")
            .await?
            .with_value("aaa")
            .await?
            .bind("bar")
            .await?
            .with_value(123)
            .await?
            .bind("void")
            .await?
            .without_value()
            .await?;

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{3}test\u{0}foo\u{0}aaa\u{0}xs:string\u{0}\
            \u{3}test\u{0}bar\u{0}123\u{0}xs:int\u{0}\
            \u{3}test\u{0}void\u{0}\u{0}\u{0}"
            .to_owned();

        assert_eq!(expected_buffer, actual_buffer);
        Ok(())
    }

    #[tokio::test]
    async fn test_query_fails_to_bind_argument_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.bind("foo").await.err().expect("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_query_binds_value_to_context() {
        let connection = Connection::from_str("\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let _ = query.context("aaa").await.unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}aaa\u{0}document-node()\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_binds_value_to_context_with_empty_type() {
        let connection = Connection::from_str("\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let _ = query.context("aaa").await.unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}aaa\u{0}document-node()\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_binds_empty_value_to_context() {
        let connection = Connection::from_str("\0\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let _ = query.context(&mut empty()).await.unwrap();

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{e}test\u{0}\u{0}document-node()\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_fails_to_bind_context_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.context(&mut empty()).await.err().expect("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_query_executes() {
        let expected_response = "test_response";
        let connection = Connection::from_str(expected_response.to_owned() + "\0");

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let mut actual_response = String::new();
        let mut response = query.execute().await.unwrap();
        response.read_to_string(&mut actual_response).await.unwrap();

        assert_eq!(expected_response, actual_response);

        let query = response.close().await.unwrap();
        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{5}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_fails_to_execute_with_failing_stream() {
        let connection = Connection::failing();

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.execute().await.err().expect("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_query_runs_updating_command() {
        let connection = Connection::from_str("true\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.updating().await.unwrap();

        assert!(actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{1e}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_runs_non_updating_command() {
        let connection = Connection::from_str("false\0");

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.updating().await.unwrap();

        assert!(!actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{1e}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_query_panics_updating_command_response_is_not_bool() {
        let connection = Connection::from_str("test_response\0");
        let client = Client::new(connection);

        let mut query = Query::with_info("test".to_owned(), client);
        let _ = query.updating().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_fails_to_run_updating_command_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.updating().await.expect_err("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_query_runs_options_command() {
        let expected_response = "ident=no";
        let connection = Connection::from_str(&format!("{}\0\0", expected_response));

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.options().await.unwrap();

        assert_eq!(expected_response, &actual_response.to_string());

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{7}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_fails_to_run_options_command_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.options().await.expect_err("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_query_runs_info_command() {
        let expected_response = QUERY_INFO;
        let connection = Connection::from_str(&format!("{}\0\0", expected_response));

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_response = query.info().await.unwrap();

        assert_query_info!(actual_response);

        let stream = query.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{6}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_fails_to_run_info_command_with_failing_stream() {
        let connection = Connection::failing();

        let mut query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.info().await.expect_err("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }

    #[tokio::test]
    async fn test_query_closes() {
        let expected_response = "test_response\0";
        let connection = Connection::from_str(expected_response);

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let client = query.close().await.unwrap();

        let stream = client.into_inner().into_inner();
        let actual_buffer = stream.to_string();
        let expected_buffer = "\u{2}test\u{0}".to_owned();

        assert_eq!(expected_buffer, actual_buffer);
    }

    #[tokio::test]
    async fn test_query_fails_to_close_with_failing_stream() {
        let connection = Connection::failing();

        let query = Query::with_info("test".to_owned(), Client::new(connection));
        let actual_error = query.close().await.err().expect("Operation must fail");

        assert!(matches!(actual_error, ConnectionError::Io(_)));
    }
}
