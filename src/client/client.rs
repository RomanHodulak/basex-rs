use std::borrow::{Borrow, BorrowMut};
use crate::{Result, Connection, Query, DatabaseStream};
use std::net::TcpStream;
use crate::client::Response;
use crate::connection::Authenticated;
use crate::resource::AsResource;

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
pub struct CommandWithOptionalInput<'a, T> where T: DatabaseStream {
    connection: &'a mut Connection<T, Authenticated>,
}

impl<'a, T> CommandWithOptionalInput<'a, T> where T: DatabaseStream {
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
/// # Example
/// ```
/// use basex::{Client, ClientError, Connection};
///
/// # fn main() -> Result<(), ClientError> {
/// use std::io::Read;
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// let info = client.create("a45d766")?
///     .with_input("<Root><Text></Text><Lala></Lala><Papa></Papa></Root>")?;
/// assert!(info.starts_with("Database 'a45d766' created"));
///
/// let query = client.query("count(/Root/*)")?;
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
pub struct Client<T> where T: DatabaseStream {
    connection: Connection<T, Authenticated>,
}

impl Client<TcpStream> {
    /// Connects and authenticates to BaseX server using TCP stream.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    ///
    /// # fn main() -> Result<(), ClientError> {
    /// let client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client<TcpStream>> {
        let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
        let connection = Connection::new(stream)
            .authenticate(user, password)?;

        Ok(Client::new(connection))
    }
}

impl<T> Client<T> where T: DatabaseStream {
    /// Returns new client instance with the given connection bound to it. It assumes that the connection is
    /// authenticated.
    ///
    /// Typically, you only need to use this method when using a custom connection. It is used heavily in tests, for
    /// example. For regular usage, refer to the [`Client::connect`] method.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError, Connection};
    /// use std::net::TcpStream;
    ///
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

    /// Executes a [database command](https://docs.basex.org/wiki/Commands).
    ///
    /// # Arguments
    /// * `command` DB command to execute including arguments.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    /// use std::io::Read;
    ///
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut list = String::new();
    /// client.execute("LIST")?.read_to_string(&mut list)?;
    /// println!("{}", list);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute(mut self, command: &str) -> Result<Response<T>> {
        self.connection.send_arg(&mut command.as_bytes())?;
        Ok(Response::new(self))
    }

    /// Creates a new database with the specified name and, optionally, an initial input, and opens it. An existing
    /// database will be overwritten. The input can be any stream pointing to a valid XML.
    ///
    /// Database creation can be controlled by setting [Create Options](http://docs.basex.org/wiki/Options#Create_Options)
    ///
    /// # Arguments
    /// * `name` must be a [valid database name](http://docs.basex.org/wiki/Commands#Valid_Names)
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    ///
    /// # fn main() -> Result<(), ClientError> {
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// client.create("boy_sminem")?.with_input("<wojak pink_index=\"69\"></wojak>")?;
    /// client.create("bogdanoff")?.without_input()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create(&mut self, name: &str) -> Result<CommandWithOptionalInput<T>> {
        self.connection.send_cmd(Command::Create as u8)?;
        self.connection.send_arg(&mut name.as_bytes())?;
        Ok(CommandWithOptionalInput::new(&mut self.connection))
    }

    /// Replaces resources in the currently opened database, addressed by path, with the XML document specified by
    /// input, or adds new documents if no resource exists at the specified path.
    ///
    /// # Arguments
    /// * `path` a path to put the input at in the currently opened database.
    /// * `input` a stream with XML data.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    ///
    /// # fn main() -> Result<(), ClientError> {
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

    /// Stores a binary file specified via input in the currently opened database to the specified
    /// path. An existing resource will be replaced.
    ///
    /// # Arguments
    /// * `path` a path to put the input at in the currently opened database.
    /// * `input` a stream with XML data.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    ///
    /// # fn main() -> Result<(), ClientError> {
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

    /// Adds an XML resource to the currently opened database at the specified path. Note that:
    /// *  A document with the same path may occur more than once in a database. If this is unwanted, the
    /// `Client::replace` method can be used.
    /// *  If the stream is too large to be added in one go, its data structures will be cached to disk first.
    /// Caching can be enforced by turning the `ADDCACHE` option on.
    ///
    /// # Arguments
    /// * `path` a path to put the input at in the currently opened database.
    /// * `input` a stream with XML data.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    ///
    /// # fn main() -> Result<(), ClientError> {
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

    /// Creates new query instance from given XQuery string.
    ///
    /// # Arguments
    /// * `query` a stream with XQuery data.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError};
    ///
    /// # fn main() -> Result<(), ClientError> {
    /// use std::io::Read;
    /// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///
    /// let info = client.create("triangle")?
    ///     .with_input("<polygon><line></line><line></line><line></line></polygon>")?;
    /// assert!(info.starts_with("Database 'triangle' created"));
    ///
    /// let query = client.query("count(/polygon/*)")?;
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
    pub fn query<'a>(mut self, query: impl AsResource<'a>) -> Result<Query<T>> {
        self.connection.send_cmd(Command::Query as u8)?;
        self.connection.send_arg(&mut query.into_read())?;
        let id = self.connection.get_response()?;

        Ok(Query::new(id, self))
    }
}

impl<T: DatabaseStream> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.try_clone().unwrap(),
        }
    }
}

impl<T: DatabaseStream> Borrow<Connection<T, Authenticated>> for Client<T> {
    fn borrow(&self) -> &Connection<T, Authenticated> {
        &self.connection
    }
}

impl<T: DatabaseStream> BorrowMut<Connection<T, Authenticated>> for Client<T> {
    fn borrow_mut(&mut self) -> &mut Connection<T, Authenticated> {
        &mut self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientError;
    use crate::tests::MockStream;

    impl<T> Client<T> where T: DatabaseStream {
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
    fn test_borrows_as_connection() {
        let _: &Connection<MockStream, Authenticated> = Client::new(
            Connection::from_str("test")
        ).borrow();
    }

    #[test]
    fn test_database_is_created_with_input() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client.create("boy_sminem").unwrap()
            .with_input("<wojak><pink_index>69</pink_index></wojak>").unwrap();

        assert_eq!(client.into_inner().into_inner().to_string(), "\u{8}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_database_is_created_without_input() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client.create("boy_sminem").unwrap()
            .without_input().unwrap();

        assert_eq!(client.into_inner().into_inner().to_string(), "\u{8}boy_sminem\u{0}\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_database_fails_to_create_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client.create("boy_sminem")
            .err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_replaced() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client.replace("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>").unwrap();

        assert_eq!(client.into_inner().into_inner().to_string(), "\u{c}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_replace_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client.replace("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_stored() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client.store("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>").unwrap();

        assert_eq!(client.into_inner().into_inner().to_string(), "\u{d}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_store_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client.store("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_added() {
        let mut client = Client::new(Connection::from_str("test\0"));

        let info = client.add("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>").unwrap();

        assert_eq!(client.into_inner().into_inner().to_string(), "\u{9}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_add_with_failing_stream() {
        let mut client = Client::new(Connection::failing());

        let actual_error = client.add("boy_sminem", "<wojak><pink_index>69</pink_index></wojak>")
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }
}
