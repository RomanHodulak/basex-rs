use std::net::TcpStream;
use std::io::{Write, Read, Error};
use std::string::FromUtf8Error;
use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug)]
pub enum ClientError {
    Io(Error),
    Utf8Parse(FromUtf8Error),
    Auth,
    CommandFailed {
        message: String,
    },
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &*self {
            ClientError::Io(ref e) => e.fmt(f),
            ClientError::Utf8Parse(ref e) => e.fmt(f),
            ClientError::Auth => write!(f, "Access denied."),
            ClientError::CommandFailed { message } => write!(f, "{}", message),
        }
    }
}

impl From<Error> for ClientError {
    fn from(err: Error) -> ClientError {
        ClientError::Io(err)
    }
}

impl From<FromUtf8Error> for ClientError {
    fn from(err: FromUtf8Error) -> ClientError {
        ClientError::Utf8Parse(err)
    }
}

/// Connects and authenticates to BaseX server.
pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Client> {
    let mut stream = TcpStream::connect(&format!("{}:{}", host, port))?;
    let mut connection = Connection::new(stream);
    connection.authenticate(host, port, user, password)?;

    Ok(Client::new(connection))
}

pub struct Connection {
    stream: TcpStream,
}

impl Connection {

    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    fn authenticate(&mut self, host: &str, port: u16, user: &str, password: &str) -> Result<&Self> {
        let response = self.read_string()?;

        let challenge: Vec<&str> = response.split(":").collect();
        let server_name = challenge[0];
        let timestamp = challenge[1];

        let first_digest = md5::compute(&format!("{}:{}:{}", user, server_name, password));
        let second_digest = md5::compute(&format!("{:x}{}", first_digest, timestamp));

        let auth_string = format!("{}\0{:x}\0", user, second_digest);
        let mut control_byte: [u8; 1] = [0];

        self.stream.write(auth_string.as_bytes())?;
        self.stream.read(&mut control_byte)?;

        if control_byte[0] != 0 {
            return Err(ClientError::Auth);
        }

        Ok(self)
    }

    fn read_string(&mut self) -> Result<String> {
        let mut raw_string: Vec<u8> = vec![];
        loop {
            let mut buf: [u8; 1] = [0];
            self.stream.read(&mut buf)?;

            if buf[0] == 0 {
                break;
            }
            raw_string.push(buf[0]);
        }

        Ok(String::from_utf8(raw_string)?)
    }

    /// Sends command identified by the code and supplies the given arguments.
    fn send_cmd(&mut self, code: u8, arguments: Vec<Option<&str>>) -> Result<&Self> {
        let mut data: Vec<u8> = vec![code];

        for argument in arguments {
            if argument.is_some() {
                data.extend_from_slice(argument.unwrap().as_bytes());
            }
            data.push(0);
        }

        self.stream.write(&data)?;

        Ok(self)
    }

    /// Gets response string, and returns string if command was successful. Returns `CommandFailed`
    /// error with a message otherwise.
    fn get_response(&mut self) -> Result<String> {
        let info = self.read_string()?;

        if self.is_ok()? {
            Ok(info)
        }
        else {
            Err(ClientError::CommandFailed { message: info })
        }
    }

    /// Reads return code and decodes it to TRUE on success or FALSE on error.
    fn is_ok(&mut self) -> Result<bool> {
        let mut buf: [u8; 1] = [0];
        self.stream.read(&mut buf)?;

        Ok(buf[0] == 0)
    }

    /// Creates a new connection with a new independently owned handle to the underlying socket.
    fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?
        })
    }
}

pub struct Client {
    connection: Connection,
}

impl Client {
    const QUERY_CODE: u8 = 0;
    const CREATE_CODE: u8 = 8;
    const ADD_CODE: u8 = 9;
    const REPLACE_CODE: u8 = 12;
    const STORE_CODE: u8 = 13;

    /// Returns new client instance with the TCP stream bound to it. It assumes that the stream is
    /// connected and authenticated to BaseX server. Unless you need to supply your own stream for
    /// some reason, instead of calling this use the factory method. Example:
    /// ```rust
    /// let client = basex::connect("localhost", 8984, "admin", "admin");
    /// ```
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Creates a new database with the specified name and, optionally, an initial input, and opens
    /// it. An existing database will be overwritten. The input can be a file or directory path to
    /// XML documents, a remote URL, or a string containing XML.
    /// *  `name` must be a [http://docs.basex.org/wiki/Commands#Valid_Names](valid database name)
    /// *  database creation can be controlled by setting [http://docs.basex.org/wiki/Options#Create_Options](Create Options)
    pub fn create(&mut self, name: &str, input: Option<&str>) -> Result<String> {
        self.connection.send_cmd(Self::CREATE_CODE, vec![Some(name), input])?;
        self.connection.get_response()
    }

    /// Replaces resources in the currently opened database, addressed by path, with the file,
    /// directory or XML string specified by input, or adds new documents if no resource exists at
    /// the specified path.
    pub fn replace(&mut self, path: &str, input: Option<&str>) -> Result<String> {
        self.connection.send_cmd(Self::REPLACE_CODE, vec![Some(path), input])?;
        self.connection.get_response()
    }

    /// Stores a binary file specified via input in the currently opened database to the specified
    /// path.
    /// *  The input may either be a file reference, a remote URL, or a plain string.
    /// *  If the path denotes a directory, it needs to be suffixed with a slash (/).
    /// *  An existing resource will be replaced.
    pub fn store(&mut self, path: &str, input: Option<&str>) -> Result<String> {
        self.connection.send_cmd(Self::STORE_CODE, vec![Some(path), input])?;
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
    pub fn add(&mut self, path: &str, input: Option<&str>) -> Result<String> {
        self.connection.send_cmd(Self::ADD_CODE, vec![Some(path), input])?;
        self.connection.get_response()
    }

    /// Creates new query instance from given XQuery string.
    pub fn query(&mut self, query: &str) -> Result<Query> {
        self.connection.send_cmd(Self::QUERY_CODE, vec![Some(query)])?;
        let id = self.connection.get_response()?;

        Ok(Query::new(id, self.connection.try_clone()?))
    }
}

pub struct Query {
    id: String,
    connection: Connection,
}

impl Query {
    const CLOSE_CODE: u8 = 2;
    const BIND_CODE: u8 = 3;
    const RESULTS_CODE: u8 = 4;
    const EXECUTE_CODE: u8 = 5;
    const INFO_CODE: u8 = 6;
    const OPTIONS_CODE: u8 = 7;
    const CONTEXT_CODE: u8 = 0x0e;
    const UPDATING_CODE: u8 = 0x1e;

    pub fn new(id: String, connection: Connection) -> Self {
        Self { id, connection }
    }

    pub fn close(&mut self) -> Result<&mut Self> {
        self.connection.send_cmd(Self::CLOSE_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn bind(&mut self, name: &str, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Self::BIND_CODE, vec![Some(&self.id), Some(name), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn execute(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::EXECUTE_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::INFO_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::OPTIONS_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn context(&mut self, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Self::CONTEXT_CODE, vec![Some(&self.id), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn updating(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::UPDATING_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }
}
