use crate::{Result, Connection, Query, DatabaseStream};
use std::net::TcpStream;
use std::io::Read;

/// Represents database command code in the [standard mode](https://docs.basex.org/wiki/Standard_Mode).
enum Command {
    Query = 0,
    Create = 8,
    Add = 9,
    Replace = 12,
    Store = 13,
}

/// Encapsulates a command with optional input. To execute it, either call `with_input` or `without_input`.
pub struct CommandWithOptionalInput<'a, T> where T: DatabaseStream {
    client: &'a mut Client<T>,
}

impl<'a, T> CommandWithOptionalInput<'a, T> where T: DatabaseStream {
    fn new(client: &'a mut Client<T>) -> Self {
        Self { client }
    }

    /// Sends the input to the command and executes it, returning its response as a string.
    pub fn with_input<R: Read>(self, input: &mut R) -> Result<String> {
        self.client.connection.send_arg(input)?;
        self.client.connection.get_response()
    }

    /// Omits the input from command and executes it, returning its response as a string.
    pub fn without_input(self) -> Result<String> {
        self.client.connection.skip_arg()?;
        self.client.connection.get_response()
    }
}

/// Represents an interface to communicate with the BaseX server. Its main purpose is to send database
/// [commands](https://docs.basex.org/wiki/Commands) and create [queries](https://docs.basex.org/wiki/XQuery).
///
/// Start by connecting to the database using `Client::connect`.
///
/// # Example
/// ```rust
/// use basex::{Client, ClientError, Connection};
///
/// fn main() -> Result<(), ClientError> {
///     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
///     let info = client.create("lambada")?
///         .with_input(&mut "<Root><Text></Text><Lala></Lala><Papa></Papa></Root>".as_bytes())?;
///     assert!(info.starts_with("Database 'lambada' created"));
///
///     let mut query = client.query(&mut "count(/Root/*)".as_bytes())?;
///     let result = query.execute()?;
///     assert_eq!(result, "3");
///
///     let _ = query.close()?;
///     Ok(())
/// }
/// ```
pub struct Client<T> where T: DatabaseStream {
    connection: Connection<T>,
}

impl Client<TcpStream> {
    /// Connects and authenticates to BaseX server using TCP stream.
    ///
    /// # Example
    /// ```rust
    /// use basex::Client;
    ///
    /// let client = Client::connect("localhost", 1984, "admin", "admin");
    /// ```
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client<TcpStream>> {
        let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
        let mut connection = Connection::new(stream);
        connection.authenticate(user, password)?;

        Ok(Client::new(connection))
    }
}

impl<T> Client<T> where T: DatabaseStream {

    /// Returns new client instance with the given connection bound to it. It assumes that the connection is
    /// authenticated.
    ///
    /// Typically, you only need to use this method when using a custom connection. It is used heavily in tests, for
    /// example. For regular usage, refer to the `Client::connect` method.
    ///
    /// # Example
    /// ```rust
    /// use basex::{Client, ClientError, Connection};
    /// use std::net::TcpStream;
    ///
    /// fn main() -> Result<(), ClientError> {
    ///     let stream = TcpStream::connect("localhost:1984")?;
    ///     let mut connection = Connection::new(stream);
    ///     connection.authenticate("admin", "admin")?;
    ///
    ///     let client = Client::new(connection);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(connection: Connection<T>) -> Self {
        Self { connection }
    }

    /// Creates a new database with the specified name and, optionally, an initial input, and opens it. An existing
    /// database will be overwritten. The input can be any stream pointing to a valid XML.
    ///
    /// Database creation can be controlled by setting [Create Options](http://docs.basex.org/wiki/Options#Create_Options)
    ///
    /// # Arguments
    /// *  `name` must be a [valid database name](http://docs.basex.org/wiki/Commands#Valid_Names)
    ///
    /// # Example
    /// ```rust
    /// use basex::{Client, ClientError};
    ///
    /// fn main() -> Result<(), ClientError> {
    ///     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///     let _ = client.create("boy_sminem")?.with_input(&mut "<wojak pink_index=\"69\"></wojak>".as_bytes())?;
    ///     let _ = client.create("bogdanoff")?.without_input()?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn create(&mut self, name: &str) -> Result<CommandWithOptionalInput<T>> {
        self.connection.send_cmd(Command::Create as u8)?;
        self.connection.send_arg(&mut name.as_bytes())?;
        Ok(CommandWithOptionalInput::new(self))
    }

    /// Replaces resources in the currently opened database, addressed by path, with the XML document specified by
    /// input, or adds new documents if no resource exists at the specified path.
    ///
    /// # Arguments
    /// *  `path` a path to put the input at in the currently opened database.
    /// *  `input` a stream with XML data.
    ///
    /// # Example
    /// ```rust
    /// use basex::{Client, ClientError};
    ///
    /// fn main() -> Result<(), ClientError> {
    ///     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///     let _ = client.create("bell")?.without_input()?;
    ///     let _ = client.replace("bogdanoff", &mut "<wojak pink_index=\"69\"></wojak>".as_bytes())?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn replace<R: Read>(&mut self, path: &str, input: &mut R) -> Result<String> {
        self.connection.send_cmd(Command::Replace as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(input)?;
        self.connection.get_response()
    }

    /// Stores a binary file specified via input in the currently opened database to the specified
    /// path. An existing resource will be replaced.
    ///
    /// # Arguments
    /// *  `path` a path to put the input at in the currently opened database.
    /// *  `input` a stream with XML data.
    ///
    /// # Example
    /// ```rust
    /// use basex::{Client, ClientError};
    ///
    /// fn main() -> Result<(), ClientError> {
    ///     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///     let mut blob = [0 as u8, 1, 2, 3];
    ///     let _ = client.create("asylum")?.without_input()?;
    ///     let _ = client.store("bogdanoff", &mut &blob[..])?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn store<R: Read>(&mut self, path: &str, input: &mut R) -> Result<String> {
        self.connection.send_cmd(Command::Store as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(input)?;
        self.connection.get_response()
    }

    /// Adds an XML resource to the currently opened database at the specified path. Note that:
    /// *  A document with the same path may occur more than once in a database. If this is unwanted, the
    /// `Client::replace` method can be used.
    /// *  If the stream is too large to be added in one go, its data structures will be cached to disk first.
    /// Caching can be enforced by turning the `ADDCACHE` option on.
    ///
    /// # Arguments
    /// *  `path` a path to put the input at in the currently opened database.
    /// *  `input` a stream with XML data.
    ///
    /// # Example
    /// ```rust
    /// use basex::{Client, ClientError};
    ///
    /// fn main() -> Result<(), ClientError> {
    ///     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///     let _ = client.create("taurus")?.without_input()?;
    ///     let _ = client.add("bogdanoff", &mut "<wojak pink_index=\"69\"></wojak>".as_bytes())?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn add<R: Read>(&mut self, path: &str, input: &mut R) -> Result<String> {
        self.connection.send_cmd(Command::Add as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(input)?;
        self.connection.get_response()
    }

    /// Creates new query instance from given XQuery string.
    ///
    /// # Arguments
    /// *  `query` a stream with XQuery data.
    ///
    /// # Example
    /// ```rust
    /// use basex::{Client, ClientError};
    ///
    /// fn main() -> Result<(), ClientError> {
    ///     let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
    ///
    ///     let info = client.create("triangle")?
    ///         .with_input(&mut "<polygon><line></line><line></line><line></line></polygon>".as_bytes())?;
    ///     assert!(info.starts_with("Database 'triangle' created"));
    ///
    ///     let mut query = client.query(&mut "count(/polygon/*)".as_bytes())?;
    ///     let result = query.execute()?;
    ///     assert_eq!(result, "3");
    ///
    ///     let _ = query.close()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn query<R: Read>(&mut self, query: &mut R) -> Result<Query<T>> {
        self.connection.send_cmd(Command::Query as u8)?;
        self.connection.send_arg(query)?;
        let id = self.connection.get_response()?;

        Ok(Query::new(id, self.connection.try_clone()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{MockStream, FailingStream};
    use crate::ClientError;

    #[test]
    fn test_database_is_created_with_input() {
        let mut stream = MockStream::new("test".to_owned());
        let mut client = Client::new(Connection::new(stream.try_clone().unwrap()));

        let info = client.create("boy_sminem").unwrap()
            .with_input(&mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes()).unwrap();

        assert_eq!(stream.to_string(), "\u{8}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_database_is_created_without_input() {
        let mut stream = MockStream::new("test".to_owned());
        let mut client = Client::new(Connection::new(stream.try_clone().unwrap()));

        let info = client.create("boy_sminem").unwrap()
            .without_input().unwrap();

        assert_eq!(stream.to_string(), "\u{8}boy_sminem\u{0}\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_database_fails_to_create_with_failing_stream() {
        let mut client = Client::new(Connection::new(FailingStream));

        let actual_error = client.create("boy_sminem")
            .err().expect("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_replaced() {
        let mut stream = MockStream::new("test".to_owned());
        let mut client = Client::new(Connection::new(stream.try_clone().unwrap()));

        let info = client.replace("boy_sminem", &mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes()).unwrap();

        assert_eq!(stream.to_string(), "\u{c}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_replace_with_failing_stream() {
        let mut client = Client::new(Connection::new(FailingStream));

        let actual_error = client.replace("boy_sminem", &mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes())
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_stored() {
        let mut stream = MockStream::new("test".to_owned());
        let mut client = Client::new(Connection::new(stream.try_clone().unwrap()));

        let info = client.store("boy_sminem", &mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes()).unwrap();

        assert_eq!(stream.to_string(), "\u{d}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_store_with_failing_stream() {
        let mut client = Client::new(Connection::new(FailingStream));

        let actual_error = client.store("boy_sminem", &mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes())
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }

    #[test]
    fn test_resource_is_added() {
        let mut stream = MockStream::new("test".to_owned());
        let mut client = Client::new(Connection::new(stream.try_clone().unwrap()));

        let info = client.add("boy_sminem", &mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes()).unwrap();

        assert_eq!(stream.to_string(), "\u{9}boy_sminem\u{0}<wojak><pink_index>69</pink_index></wojak>\u{0}".to_owned());
        assert_eq!("test", info);
    }

    #[test]
    fn test_resource_fails_to_add_with_failing_stream() {
        let mut client = Client::new(Connection::new(FailingStream));

        let actual_error = client.add("boy_sminem", &mut "<wojak><pink_index>69</pink_index></wojak>".as_bytes())
            .expect_err("Operation must fail");

        assert!(matches!(actual_error, ClientError::Io(_)));
    }
}
