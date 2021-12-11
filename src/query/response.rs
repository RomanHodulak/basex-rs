use crate::{Connection, DatabaseStream, Result, Query};
use std::io::Read;
use crate::connection::Authenticated;
use crate::errors::ClientError;
use crate::query::QueryFailed;

/// Response from a command. Depending on the command, it may or may not return UTF-8 string. Result is read using
/// the [`Read`] trait.
///
/// # Example
/// ```
/// use basex::{Client, ClientError, Connection};
/// use std::io::Read;
///
/// # fn main() -> Result<(), ClientError> {
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// let info = client.create("shovels")?.without_input()?;
/// assert!(info.starts_with("Database 'shovels' created"));
/// client.store("blob", &mut &[0u8, 1, 2, 3, 4][..])?;
///
/// let mut result: Vec<u8> = vec![];
/// let mut response = client.execute("RETRIEVE blob")?;
/// response.read_to_end(&mut result)?;
///
/// let (mut client, info) = response.close()?;
/// assert!(info.starts_with("Query executed in"));
///
/// let (mut client, _) = client.execute("OPEN shovels")?.close()?;
/// client.execute("CLOSE")?.close()?;
/// # Ok(())
/// # }
/// ```
///
/// [`Read`]: std::io::Read
pub struct Response<T> where T: DatabaseStream {
    id: String,
    connection: Connection<T, Authenticated>,
    info_prefix: Option<Vec<u8>>,
    info_complete: bool,
    is_ok: bool,
    result_complete: bool,
}

impl<T> Response<T> where T: DatabaseStream {
    pub(crate) fn new(id: String, connection: Connection<T, Authenticated>) -> Self {
        Self { id, connection, info_prefix: None, info_complete: false, is_ok: false, result_complete: false, }
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
    pub fn close(mut self) -> Result<Query<T>> {
        let mut buf = [0u8; 4096];

        while !self.result_complete && self.read(&mut buf)? > 0 {}

        if !self.result_complete {
            panic!("Unexpected end of stream.");
        }

        match self.is_ok {
            true => Ok(Query::new(self.id, self.connection)),
            false => {
                let mut info = String::from_utf8(self.info_prefix.unwrap_or_default())?;

                if !self.info_complete {
                    let info_suffix = self.connection.read_string()?;
                    info.push_str(&info_suffix);
                }

                Err(ClientError::QueryFailed(QueryFailed::new(info)))
            },
        }
    }
}

impl<T> Read for Response<T> where T: DatabaseStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.result_complete {
            return Ok(0);
        }

        let size = self.connection.read(buf)?;
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
                self.result_complete = true;
                self.is_ok = match buf[..size][position + 1] {
                    0 => true,
                    1 => false,
                    other => panic!("Invalid status byte \"{}\"", other)
                };
                if self.is_ok {
                    self.info_complete = true;
                } else {
                    self.info_prefix = match buf[position + 2..size].iter().position(|&b| b == 0) {
                        Some(length) => {
                            self.info_complete = true;
                            Some(buf[position + 2..position + 2 + length].to_vec())
                        },
                        None => Some(buf[position + 2..size].to_vec()),
                    };
                }
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
    fn test_reading_result_from_response() {
        let connection = Connection::from_str("result\0".to_owned());
        let mut response = Response::new("1".to_owned(), connection);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).unwrap();
        let expected_response = "result".to_owned();

        assert_eq!(expected_response, actual_response);
    }

    #[test]
    fn test_reading_result_from_response_on_multiple_read_calls() {
        let connection = Connection::from_str("result".repeat(10) + "\0");
        let mut response = Response::new("1".to_owned(), connection);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).unwrap();
        let expected_response = "result".repeat(10).to_owned();

        assert_eq!(expected_response, actual_response);

        response.close().expect("Operation must succeed.");
    }

    #[test]
    fn test_reading_result_from_response_with_some_escape_bytes() {
        let connection = Connection::from_bytes(&[0xFFu8, 0, 1, 6, 9, 0xFF, 0xFF, 3, 0]);
        let mut response = Response::new("1".to_owned(), connection);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).unwrap();
        let expected_response = vec![0u8, 1, 6, 9, 0xFF, 3];

        assert_eq!(expected_response, actual_response);

        response.close().expect("Operation must succeed.");
    }

    #[test]
    fn test_reading_result_from_response_with_only_escape_bytes() {
        let mut bytes = [0xFFu8, 0].repeat(10);
        bytes.extend([0]);
        let connection = Connection::from_bytes(&bytes);
        let mut response = Response::new("1".to_owned(), connection);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).unwrap();
        let expected_response = [0u8].repeat(10).to_vec();

        assert_eq!(expected_response, actual_response);

        response.close().expect("Operation must succeed.");
    }

    #[test]
    fn test_reading_error_from_response() {
        let expected_error = "Stopped at ., 1/1:\n[XPST0008] Undeclared variable: $x.";
        let connection = Connection::from_str(format!("partial_result\0\u{1}{}\0", expected_error));
        let response = Response::new("1".to_owned(), connection);
        let actual_error = response.close().err().unwrap();

        assert!(matches!(
            actual_error,
            ClientError::QueryFailed(q) if q.raw() == expected_error
        ));
    }

    #[test]
    fn test_reading_error_from_response_on_multiple_read_calls() {
        let expected_error = "Stopped at ., 1/1:\n[XPST0008] ".to_owned() + &"error".repeat(5000);
        let connection = Connection::from_str(format!("partial_result\0\u{1}{}\0", expected_error));
        let response = Response::new("1".to_owned(), connection);
        let actual_error = response.close().err().unwrap();

        assert!(matches!(
            actual_error,
            ClientError::QueryFailed(q) if q.raw() == expected_error
        ));
    }

    #[test]
    #[should_panic]
    fn test_reading_panics_on_invalid_status_byte() {
        let connection = Connection::from_str("partial_result\0\u{2}test_error\0".to_owned());

        let _ = Response::new("1".to_owned(), connection).read(&mut [0u8; 27]);
    }

    #[test]
    #[should_panic]
    fn test_reading_panics_on_incomplete_result() {
        let connection = Connection::from_str("partial_result".to_owned());

        let _ = Response::new("1".to_owned(), connection).close();
    }
}
