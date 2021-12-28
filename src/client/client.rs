use crate::client::Response;
use crate::connection::{Authenticated, HasConnection};
use crate::query::{WithInfo, WithoutInfo};
use crate::resource::AsResource;
use crate::{Connection, DatabaseStream, Query, Result};
use std::marker::PhantomData;
use std::net::TcpStream;

/// Represents database command code in the [standard mode](https://docs.basex.org/wiki/Standard_Mode).
enum Command {
    Query = 0,
    Create = 8,
    Add = 9,
    Replace = 12,
    Store = 13,
}

/// Encapsulates a command with optional input. To execute it, either call [`with_input`] or [`without_input`].
///
/// [`with_input`]: self::CommandWithOptionalInput::with_input
/// [`without_input`]: self::CommandWithOptionalInput::without_input
#[derive(Debug)]
pub struct CommandWithOptionalInput<'a, T>
where
    T: DatabaseStream,
{
    connection: &'a mut Connection<T, Authenticated>,
}

impl<'a, T> CommandWithOptionalInput<'a, T>
where
    T: DatabaseStream,
{
    fn new(connection: &'a mut Connection<T, Authenticated>) -> Self {
        Self { connection }
    }

    /// Sends the input to the command and executes it, returning its response as a string.
    pub fn with_input<'b, R: AsResource<'b>>(self, input: R) -> Result<String> {
        self.connection.send_arg(&mut input.into_read())?;
        self.connection.get_response()
    }

    /// Omits the input from command and executes it, returning its response as a string.
    pub fn without_input(self) -> Result<String> {
        self.connection.skip_arg()?;
        self.connection.get_response()
    }
}

/// Represents an interface to communicate with the BaseX server. Its main purpose is to send database
/// [commands](https://docs.basex.org/wiki/Commands) and create [queries](https://docs.basex.org/wiki/XQuery).
///
/// Start by connecting to the database using [`Client::connect`].
///
/// # Examples
///
/// ```
/// # use basex::{Client, ClientError, Connection};
/// # use std::io::Read;
/// # fn main() -> Result<(), ClientError> {
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// let info = client.create("a45d766")?
///     .with_input("<Root><Text></Text><Lala></Lala><Papa></Papa></Root>")?;
/// assert!(info.starts_with("Database 'a45d766' created"));
///
/// let query = client.query("count(/Root/*)")?.without_info()?;
/// let mut result = String::new();
/// let mut response = query.execute()?;
/// response.read_to_string(&mut result);
/// assert_eq!(result, "3");
///
/// let mut query = response.close()?;
/// query.close()?;
/// # Ok(())
/// # }
/// ```
///
/// [`Client::connect`]: crate::client::Client<TcpStream>::connect
#[derive(Debug)]
pub struct Client<T>
where
    T: DatabaseStream,
{
    connection: Connection<T, Authenticated>,
}

impl Client<TcpStream> {
    /// Connects and authenticates to BaseX server using TCP stream.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, ClientError};
    /// # fn main() -> Result<(), ClientError> {
    /// let client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client<TcpStream>> {
        let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
        let connection = Connection::new(stream).authenticate(user, password)?;

        Ok(Client::new(connection))
    }
}

impl<T> Client<T>
where
    T: DatabaseStream,
{
    /// Returns new client instance with the given connection bound to it. Works only with authenticated connections.
    ///
    /// Typically, you only need to use this method when using a custom connection. It is used heavily in tests, for
    /// example. For regular usage, refer to the [`Client::connect`] method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, ClientError, Connection};
    /// # use std::net::TcpStream;
    /// # fn main() -> Result<(), ClientError> {
    /// let stream = TcpStream::connect("localhost:1984")?;
    /// let connection = Connection::new(stream).authenticate("admin", "admin")?;
    ///
    /// let client = Client::new(connection);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Client::connect`]: crate::client::Client<TcpStream>::connect
    pub fn new(connection: Connection<T, Authenticated>) -> Self {
        Self { connection }
    }

    /// Executes a server [`command`](https://docs.basex.org/wiki/Commands) including arguments.
    ///
    /// Returns response which can be read using the [`Read`] trait.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, Result};
    /// # use std::io::Read;
    /// # fn main() -> Result<()> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut list = String::new();
    /// client.execute("LIST")?.read_to_string(&mut list)?;
    /// println!("{}", list);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Read`]: std::io::Read
    pub fn execute(mut self, command: &str) -> Result<Response<T>> {
        self.connection.send_arg(&mut command.as_bytes())?;
        Ok(Response::new(self))
    }

    /// Creates a new database with the specified `name` and, optionally, an initial `input` and opens it.
    ///
    /// * Overwrites existing database with the same `name`.
    /// * The `name` must be [valid database name](http://docs.basex.org/wiki/Commands#Valid_Names).
    /// * The `input` is a stream with valid XML.
    /// * More options can be controlled by setting [Create Options](http://docs.basex.org/wiki/Options#Create_Options)
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, Result};
    /// # fn main() -> Result<()> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// client.create("boy_sminem")?.with_input("<wojak pink_index=\"69\"></wojak>")?;
    /// client.create("bogdanoff")?.without_input()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create(&mut self, name: &str) -> Result<CommandWithOptionalInput<'_, T>> {
        self.connection.send_cmd(Command::Create as u8)?;
        self.connection.send_arg(&mut name.as_bytes())?;
        Ok(CommandWithOptionalInput::new(&mut self.connection))
    }

    /// Replaces resources in the currently opened database, addressed by `path`, with the XML document read from
    /// `input`, or adds new documents if no resource exists at the specified path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, Result};
    /// # fn main() -> Result<()> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// client.create("bell")?.without_input()?;
    /// client.replace("bogdanoff", "<wojak pink_index=\"69\"></wojak>")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn replace<'a>(&mut self, path: &str, input: impl AsResource<'a>) -> Result<String> {
        self.connection.send_cmd(Command::Replace as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(&mut input.into_read())?;
        self.connection.get_response()
    }

    /// Stores a binary file from `input` in the currently opened database under `path`. Overwrites existing resource.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, Result};
    /// # fn main() -> Result<()> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut blob = [0 as u8, 1, 2, 3];
    /// client.create("asylum")?.without_input()?;
    /// client.store("bogdanoff", &mut &blob[..])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn store<'a>(&mut self, path: &str, input: impl AsResource<'a>) -> Result<String> {
        self.connection.send_cmd(Command::Store as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(&mut input.into_read())?;
        self.connection.get_response()
    }

    /// Adds an XML resource to the currently opened database under the specified `path`.
    ///
    /// * Keeps multiple documents with the same `path`. If this is unwanted, use `Client::replace`.
    /// * On the server-side if the stream is too large to be added in one go, its data structures will be cached to
    /// disk first. Caching can be enforced by turning the `ADDCACHE` option on.
    /// * The `input` is a stream with valid XML.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, Result};
    /// # fn main() -> Result<()> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// client.create("taurus")?.without_input()?;
    /// client.add("bogdanoff", &mut "<wojak pink_index=\"69\"></wojak>".as_bytes())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add<'a>(&mut self, path: &str, input: impl AsResource<'a>) -> Result<String> {
        self.connection.send_cmd(Command::Add as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(&mut input.into_read())?;
        self.connection.get_response()
    }

    /// Creates a new `query` from given XQuery code.
    ///
    /// You then need to make a statement about collecting compiler info using either [`with_info`] or [`without_info`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{Client, Result};
    /// # use std::io::Read;
    /// # fn main() -> Result<()> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///
    /// let info = client.create("triangle")?
    ///     .with_input("<polygon><line></line><line></line><line></line></polygon>")?;
    /// assert!(info.starts_with("Database 'triangle' created"));
    ///
    /// let query = client.query("count(/polygon/*)")?.without_info()?;
    /// let mut result = String::new();
    /// let mut response = query.execute()?;
    /// response.read_to_string(&mut result)?;
    /// assert_eq!(result, "3");
    ///
    /// let mut query = response.close()?;
    /// query.close()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`with_info`]: self::QueryWithOptionalInfo::with_info
    /// [`without_info`]: self::QueryWithOptionalInfo::without_info
    pub fn query<'a, R: AsResource<'a>>(self, query: R) -> Result<QueryWithOptionalInfo<'a, T, R>> {
        Ok(QueryWithOptionalInfo::new(self, query))
    }
}

impl<T: DatabaseStream> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.try_clone().unwrap(),
        }
    }
}

impl<T: DatabaseStream> HasConnection<T> for Client<T> {
    fn connection(&mut self) -> &mut Connection<T, Authenticated> {
        &mut self.connection
    }
}

#[derive(Debug)]
pub struct QueryWithOptionalInfo<'a, T, R>
where
    T: DatabaseStream,
    R: AsResource<'a>,
{
    phantom: PhantomData<&'a ()>,
    client: Client<T>,
    query: R,
}

impl<'a, T, R> QueryWithOptionalInfo<'a, T, R>
where
    T: DatabaseStream,
    R: AsResource<'a>,
{
    fn new(client: Client<T>, query: R) -> Self {
        Self {
            phantom: Default::default(),
            client,
            query,
        }
    }

    pub fn with_info(self) -> Result<Query<T, WithInfo>> {
        let (mut client, _) = self.client.execute("SET QUERYINFO true")?.close()?;
        let id = Self::query(&mut client, self.query)?;
        Ok(Query::with_info(id, client))
    }

    pub fn without_info(self) -> Result<Query<T, WithoutInfo>> {
        let (mut client, _) = self.client.execute("SET QUERYINFO false")?.close()?;
        let id = Self::query(&mut client, self.query)?;
        Ok(Query::without_info(id, client))
    }

    fn query(client: &mut Client<T>, query: R) -> Result<String> {
        client.connection.send_cmd(Command::Query as u8)?;
        client.connection.send_arg(&mut query.into_read())?;
        client.connection.get_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientError;

    impl<T> Client<T>
    where
        T: DatabaseStream,
    {
        pub(crate) fn into_inner(self) -> Connection<T, Authenticated> {
            self.connection
        }
    }

    #[test]
    fn test_formats_as_debug() {
        format!("{:?}", Client::new(Connection::failing()));
    }

    #[test]
    fn test_clones() {
        let _ = Client::new(Connection::from_str("")).clone();
    }

    #[test]
    fn test_database_is_created_with_input() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client
            .create("boy_sminem")
            .unwrap()
            .with_input("<wojak><pink_index>69</pink_index></wojak>")
            .unwrap();

        assert_eq!(
            client.into_inner().into_inner().to_string(),
            "\u{8}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned()
        );
        assert_eq!("test", info);
    }

    #[test]
    fn test_database_is_created_without_input() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client.create("boy_sminem").unwrap().without_input().unwrap();

        assert_eq!(
            client.into_inner().into_inner().to_string(),
            "\u{8}boy_sminem\u{0}\u{0}".to_owned()
        );
        assert_eq!("test", info);
    }

    #[test]
    fn test_database_fails_to_create_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client.create("boy_sminem").err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_replaced() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client
            .replace("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .unwrap();

        assert_eq!(
            client.into_inner().into_inner().to_string(),
            "\u{c}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned()
        );
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_replace_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client
            .replace("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_stored() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client
            .store("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .unwrap();

        assert_eq!(
            client.into_inner().into_inner().to_string(),
            "\u{d}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned()
        );
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_store_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client
            .store("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_added() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client
            .add("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .unwrap();

        assert_eq!(
            client.into_inner().into_inner().to_string(),
            "\u{9}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned()
        );
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_add_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client
            .add("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }
}
