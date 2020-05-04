use crate::basex::ClientError;
use super::Result;
use std::net::TcpStream;
use std::io::{Write, Read};

pub struct Connection {
    stream: TcpStream,
}

impl Connection {

    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub(crate) fn authenticate(&mut self, host: &str, port: u16, user: &str, password: &str) -> Result<&Self> {
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
    pub(crate) fn send_cmd(&mut self, code: u8, arguments: Vec<Option<&str>>) -> Result<&Self> {
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
    pub(crate) fn get_response(&mut self) -> Result<String> {
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
    pub(crate) fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?
        })
    }
}
