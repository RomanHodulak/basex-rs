use crate::connection::{Authenticated, HasConnection};
use crate::errors::ClientError::CommandFailed;
use crate::{Client, Connection, DatabaseStream, Result};
use std::io::Read;

/// Response from a command. Depending on the command, it may or may not return UTF-8 string. Result is read using
/// the [`Read`] trait.
///
/// # Example
///
/// ```
/// # use basex::{Client, ClientError, Connection};
/// # use std::io::Read;
/// # fn main() -> Result<(), ClientError> {
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// let info = client.create("9f8fe63")?.without_input()?;
/// assert!(info.starts_with("Database '9f8fe63' created"));
/// client.store("blob", &mut &[0u8, 1, 2, 3, 4][..])?;
///
/// let mut result: Vec<u8> = vec![];
/// let mut response = client.execute("RETRIEVE blob")?;
/// response.read_to_end(&mut result)?;
///
/// let (mut client, info) = response.close()?;
/// assert!(info.starts_with("Query executed in"));
///
/// let (mut client, _) = client.execute("OPEN 9f8fe63")?.close()?;
/// client.execute("CLOSE")?.close()?;
/// # Ok(())
/// # }
/// ```
///
/// [`Read`]: std::io::Read
pub struct Response<T>
where
    T: DatabaseStream,
{
    client: Client<T>,
    info_prefix: Option<Vec<u8>>,
    info_complete: bool,
    is_ok: bool,
}

impl<T> Response<T>
where
    T: DatabaseStream,
{
    pub(crate) fn new(client: Client<T>) -> Self {
        Self {
            client,
            info_prefix: None,
            info_complete: false,
            is_ok: false,
        }
    }

    /// Reads info and returns back client.
    ///
    /// # Panics
    /// Panics when the stream ends before result is fully streamed.
    ///
    /// # Example
    /// ```
    /// use basex::{Client, ClientError, Connection};
    /// use std::io::Read;
    ///
    /// # fn main() -> Result<(), ClientError> {
    /// let client = Client::connect("localhost", 1984, "admin", "admin")?;
    /// let mut response = client.execute("CLOSE")?;
    /// let (client, info) = response.close()?;
    /// println!("{}", info);
    /// # Ok(())
    /// # }
    /// ```
    pub fn close(mut self) -> Result<(Client<T>, String)> {
        let mut buf = [0u8; 40];

        while self.info_prefix.is_none() && self.read(&mut buf)? > 0 {}

        if self.info_prefix.is_none() {
            panic!("Unexpected end of stream.");
        }

        let info_suffix = if !self.info_complete {
            let info_suffx = self.connection().read_string()?;
            self.is_ok = self.connection().is_ok()?;
            Some(info_suffx)
        } else {
            None
        };

        let mut info = String::from_utf8(self.info_prefix.unwrap())?;

        if let Some(info_suffix) = info_suffix {
            info.push_str(&info_suffix);
        }

        match self.is_ok {
            true => Ok((self.client, info)),
            false => Err(CommandFailed { message: info }),
        }
    }
}

impl<T: DatabaseStream> HasConnection<T> for Response<T> {
    fn connection(&mut self) -> &mut Connection<T, Authenticated> {
        self.client.connection()
    }
}

impl<T> Read for Response<T>
where
    T: DatabaseStream,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.info_prefix.is_some() {
            return Ok(0);
        }

        let size = self.connection().read(buf)?;
        let mut escape = false;
        let mut shift = 0usize;
        let mut position: Option<usize> = None;

        for i in 0..size {
            if buf[i] == 0xFF && !escape {
                escape = true;
                shift += 1;
                continue;
            }
            if buf[i] == 0 && !escape {
                position = Some(i);
                break;
            }

            escape = false;
            buf[i - shift] = buf[i];
        }

        if let Some(position) = position {
            if size > position + 1 {
                self.info_prefix = match buf[position + 1..size].iter().position(|&b| b == 0) {
                    Some(length) => {
                        self.info_complete = true;
                        self.is_ok = match buf[..size][position + 1 + length + 1] {
                            0 => true,
                            1 => false,
                            other => panic!("Invalid status byte \"{}\"", other),
                        };
                        Some(buf[position + 1..position + 1 + length].to_vec())
                    }
                    None => Some(buf[position + 1..size].to_vec()),
                };
            }

            return Ok(position - shift);
        }

        Ok(size - shift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientError;

    #[test]
    fn test_closing_returns_info() {
        let connection = Connection::from_str("result\0info\0\0");
        let client = Client::new(connection);
        let response = Response::new(client);
        let (_, actual_info) = response.close().unwrap();
        let expected_info = "info";

        assert_eq!(expected_info, actual_info);
    }

    #[test]
    fn test_closing_returns_info_on_multiple_read_calls() {
        let connection = Connection::from_str("result\0".to_owned() + &"info".repeat(20) + "\0\0");
        let client = Client::new(connection);

        let response = Response::new(client);
        let (_, actual_info) = response.close().unwrap();
        let expected_info = "info".repeat(20);

        assert_eq!(expected_info, actual_info);
    }

    #[test]
    fn test_reading_result_from_response() {
        let connection = Connection::from_str("result\0info\0\0".to_owned());
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).unwrap();
        let expected_response = "result".to_owned();

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_reading_result_from_response_on_multiple_read_calls() {
        let connection = Connection::from_str("result".repeat(10) + "\0info\0\0");
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).unwrap();
        let expected_response = "result".repeat(10).to_owned();

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_reading_result_from_response_with_some_escape_bytes() {
        let connection = Connection::from_bytes(&[0xFFu8, 0, 1, 6, 9, 0xFF, 0xFF, 3, 0, 0]);
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).unwrap();
        let expected_response = vec![0u8, 1, 6, 9, 0xFF, 3];

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_reading_result_from_response_with_only_escape_bytes() {
        let mut bytes = [0xFFu8, 0].repeat(10);
        bytes.extend([0, 0]);
        let connection = Connection::from_bytes(&bytes);
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).unwrap();
        let expected_response = [0u8].repeat(10).to_vec();

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_reading_error_from_response() {
        let connection = Connection::from_str("partial_result\0test_error\0\u{1}");
        let client = Client::new(connection);
        let response = Response::new(client);
        let actual_error = response.close().err().unwrap();

        assert!(matches!(
            actual_error,
            ClientError::CommandFailed { message } if message == "test_error"
        ));
    }

    #[test]
    #[should_panic]
    fn test_reading_panics_on_invalid_status_byte() {
        let connection = Connection::from_str("partial_result\0test_error\0\u{2}");
        let client = Client::new(connection);

        let _ = Response::new(client).read(&mut [0u8; 27]);
    }

    #[test]
    #[should_panic]
    fn test_reading_panics_on_incomplete_result() {
        let connection = Connection::from_str("partial_result");
        let client = Client::new(connection);

        let _ = Response::new(client).close();
    }
}
