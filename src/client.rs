use crate::{Result, Connection, Query, DatabaseStream};
use std::net::TcpStream;
use std::io::Read;

/// Represents database command code in the [standard mode](https://docs.basex.org/wiki/Standard_Mode).
pub enum Command {
    Query = 0,
    Create = 8,
    Add = 9,
    Replace = 12,
    Store = 13,
}

pub struct CommandWithOptionalInput<'a, T> where T: DatabaseStream {
    client: &'a mut Client<T>,
}

impl<'a, T> CommandWithOptionalInput<'a, T> where T: DatabaseStream {
    fn new(client: &'a mut Client<T>) -> Self {
        Self { client }
    }

    pub fn with_input<R: Read>(self, input: &mut R) -> Result<String> {
        self.client.connection.send_arg(input)?;
        self.client.connection.get_response()
    }

    pub fn without_input(self) -> Result<String> {
        self.client.connection.skip_arg()?;
        self.client.connection.get_response()
    }
}

pub struct Client<T> where T: DatabaseStream {
    connection: Connection<T>,
}

impl Client<TcpStream> {
    /// Connects and authenticates to BaseX server.
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client<TcpStream>> {
        let stream = TcpStream::connect(&format!("{}:{}", host, port))?;
        let mut connection = Connection::new(stream);
        connection.authenticate(user, password)?;

        Ok(Client::new(connection))
    }
}
impl<T> Client<T> where T: DatabaseStream {

    /// Returns new client instance with the TCP stream bound to it. It assumes that the stream is
    /// connected and authenticated to BaseX server. Unless you need to supply your own stream for
    /// some reason, instead of calling this use the factory method. Example:
    /// ```rust
    /// use basex::Client;
    ///
    /// let client = Client::connect("localhost", 1984, "admin", "admin");
    /// ```
    pub fn new(connection: Connection<T>) -> Self {
        Self { connection }
    }

    /// Creates a new database with the specified name and, optionally, an initial input, and opens
    /// it. An existing database will be overwritten. The input can be a file or directory path to
    /// XML documents, a remote URL, or a string containing XML.
    /// *  `name` must be a [valid database name](http://docs.basex.org/wiki/Commands#Valid_Names)
    /// *  database creation can be controlled by setting [Create Options](http://docs.basex.org/wiki/Options#Create_Options)
    pub fn create(&mut self, name: &str) -> Result<CommandWithOptionalInput<T>> {
        self.connection.send_cmd(Command::Create as u8)?;
        self.connection.send_arg(&mut name.as_bytes())?;
        Ok(CommandWithOptionalInput::new(self))
    }

    /// Replaces resources in the currently opened database, addressed by path, with the file,
    /// directory or XML string specified by input, or adds new documents if no resource exists at
    /// the specified path.
    pub fn replace<R: Read>(&mut self, path: &str, input: &mut R) -> Result<String> {
        self.connection.send_cmd(Command::Replace as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(input)?;
        self.connection.get_response()
    }

    /// Stores a binary file specified via input in the currently opened database to the specified
    /// path.
    /// *  The input may either be a file reference, a remote URL, or a plain string.
    /// *  If the path denotes a directory, it needs to be suffixed with a slash (/).
    /// *  An existing resource will be replaced.
    pub fn store<R: Read>(&mut self, path: &str, input: &mut R) -> Result<String> {
        self.connection.send_cmd(Command::Store as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(input)?;
        self.connection.get_response()
    }

    /// Adds a file, directory or XML string specified by input to the currently opened database at
    /// the specified path.
    /// *  `input` may either be a single XML document, a directory, a remote URL or a plain XML
    /// string.
    /// *  A document with the same path may occur than once in a database. If this is unwanted, the
    /// `replace` command can be used.
    /// *  If a file is too large to be added in one go, its data structures will be cached to disk
    /// first. Caching can be enforced by turning the ADDCACHE option on.
    pub fn add<R: Read>(&mut self, path: &str, input: &mut R) -> Result<String> {
        self.connection.send_cmd(Command::Add as u8)?;
        self.connection.send_arg(&mut path.as_bytes())?;
        self.connection.send_arg(input)?;
        self.connection.get_response()
    }

    /// Creates new query instance from given XQuery string.
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
