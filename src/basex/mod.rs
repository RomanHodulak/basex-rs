use std::net::TcpStream;
use std::io::{Write, Read, Error};
use std::string::FromUtf8Error;
use std::fmt::{Display, Formatter};

type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug)]
pub enum ClientError {
    Io(Error),
    Utf8Parse(FromUtf8Error),
    Auth,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match *self {
            ClientError::Io(ref e) => e.fmt(f),
            ClientError::Utf8Parse(ref e) => e.fmt(f),
            ClientError::Auth => write!(f, "Access denied.")
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
    let response = read_string(&mut stream)?;

    let challenge: Vec<&str> = response.split(":").collect();
    let server_name = challenge[0];
    let timestamp = challenge[1];

    let first_digest = md5::compute(&format!("{}:{}:{}", user, server_name, password));
    let second_digest = md5::compute(&format!("{:x}{}", first_digest, timestamp));

    let auth_string = format!("{}\0{:x}\0", user, second_digest);
    let mut control_byte: [u8; 1] = [0];

    stream.write(auth_string.as_bytes())?;
    stream.read(&mut control_byte)?;

    if control_byte[0] != 0 {
        return Err(ClientError::Auth);
    }

    Ok(Client::new(stream))
}

fn read_string(stream: &mut TcpStream) -> Result<String> {
    let mut raw_string: Vec<u8> = vec![];
    loop {
        let mut buf: [u8; 1] = [0];
        stream.read(&mut buf)?;

        if buf[0] == 0 {
            break;
        }
        raw_string.push(buf[0]);
    }

    Ok(String::from_utf8(raw_string)?)
}

pub struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}
