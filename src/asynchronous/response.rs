use crate::asynchronous::client::Client;
use crate::asynchronous::connection::{
    Authenticated, Connection, ConnectionError, ConnectionError::CommandFailed, HasConnection,
};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

/// Response from a command. Depending on the command, it may or may not return UTF-8 string. Result is read using
/// the [`Read`] trait.
///
/// # Examples
///
/// ```
/// # use basex::{Client, ClientError, Connection};
/// # use std::io::Read;
/// # fn main() -> Result<(), ClientError> {
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// let info = client.create("b07376b0")?.without_input()?;
/// assert!(info.starts_with("Database 'b07376b0' created"));
/// client.store("blob", &mut &[0u8, 1, 2, 3, 4][..])?;
///
/// let mut result: Vec<u8> = vec![];
/// let mut response = client.execute("RETRIEVE blob")?;
/// response.read_to_end(&mut result)?;
///
/// let (mut client, info) = response.close()?;
/// assert!(info.starts_with("Query executed in"));
///
/// let (mut client, _) = client.execute("OPEN b07376b0")?.close()?;
/// client.execute("CLOSE")?.close()?;
/// # Ok(())
/// # }
/// ```
///
/// [`Read`]: std::io::Read
#[derive(Debug)]
#[pin_project]
pub struct Response<T>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    #[pin]
    client: Client<T>,
    info_prefix: Option<Vec<u8>>,
    info_complete: bool,
    is_ok: bool,
}

impl<T> Response<T>
where
    T: AsyncWrite + AsyncRead + Unpin,
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
    /// # Examples
    ///
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
    pub async fn close(mut self) -> Result<(Client<T>, String), ConnectionError> {
        let mut buf = [0u8; 40];

        while self.info_prefix.is_none() && self.read(&mut buf).await? > 0 {}

        if self.info_prefix.is_none() {
            panic!("Unexpected end of stream.");
        }

        let info_suffix = if !self.info_complete {
            let this = Pin::new(&mut self);
            let mut connection = this.connection();
            let info_suffx = connection.read_string().await?;
            self.is_ok = connection.is_ok().await?;
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
            false => Err(CommandFailed(info)),
        }
    }
}

impl<T: AsyncWrite + AsyncRead + Unpin> HasConnection<T> for Response<T> {
    fn connection(self: Pin<&mut Self>) -> Pin<&mut Connection<T, Authenticated>> {
        let this = self.project();

        this.client.connection()
    }
}

impl<T> AsyncRead for Response<T>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        if self.info_prefix.is_some() {
            return Poll::Ready(Ok(()));
        }

        let capacity = buf.capacity();
        let mut inner_buf = vec![0u8; capacity];
        let mut escape = false;
        let mut shift = 0usize;
        let mut position: Option<usize> = None;
        let mut buf_unparsed = ReadBuf::new(inner_buf.as_mut_slice());
        buf_unparsed.put_slice(buf.filled());

        match self.as_mut().connection().poll_read(cx, &mut buf_unparsed)? {
            Poll::Pending => Poll::Pending,
            Poll::Ready(..) => {
                let size = buf_unparsed.filled().len();

                for i in 0..size {
                    if buf_unparsed.filled()[i] == 0xFF && !escape {
                        escape = true;
                        shift += 1;
                        continue;
                    }
                    if buf_unparsed.filled()[i] == 0 && !escape {
                        position = Some(i);
                        break;
                    }

                    escape = false;
                    if buf.filled_mut().len() > i - shift {
                        buf.filled_mut()[i - shift] = buf_unparsed.filled()[i];
                    } else {
                        buf.put_slice(&[buf_unparsed.filled()[i]; 1])
                    }
                }

                if let Some(position) = position {
                    if size > position + 1 {
                        self.info_prefix = match buf_unparsed.filled()[position + 1..size].iter().position(|&b| b == 0)
                        {
                            Some(length) => {
                                self.info_complete = true;
                                self.is_ok = match buf_unparsed.filled()[..size][position + 1 + length + 1] {
                                    0 => true,
                                    1 => false,
                                    other => panic!("Invalid status byte \"{}\"", other),
                                };
                                Some(buf_unparsed.filled()[position + 1..position + 1 + length].to_vec())
                            }
                            None => Some(buf_unparsed.filled()[position + 1..size].to_vec()),
                        };
                    }
                    // buf.set_filled(position - shift);

                    return Poll::Ready(Ok(()));
                }
                // buf.set_filled(size - shift);

                Poll::Ready(Ok(()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_closing_returns_info() {
        let connection = Connection::from_str("result\0info\0\0");
        let client = Client::new(connection);
        let response = Response::new(client);
        let (_, actual_info) = response.close().await.unwrap();
        let expected_info = "info";

        assert_eq!(expected_info, actual_info);
    }

    #[tokio::test]
    async fn test_closing_returns_info_on_multiple_read_calls() {
        let connection = Connection::from_str("result\0".to_owned() + &"info".repeat(20) + "\0\0");
        let client = Client::new(connection);

        let response = Response::new(client);
        let (_, actual_info) = response.close().await.unwrap();
        let expected_info = "info".repeat(20);

        assert_eq!(expected_info, actual_info);
    }

    #[tokio::test]
    async fn test_reading_result_from_response() {
        let connection = Connection::from_str("result\0info\0\0".to_owned());
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).await.unwrap();
        let expected_response = "result".to_owned();

        assert_eq!(expected_response, actual_response);
    }

    #[tokio::test]
    async fn test_reading_result_from_response_on_multiple_read_calls() {
        let connection = Connection::from_str("result".repeat(10) + "\0info\0\0");
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).await.unwrap();
        let expected_response = "result".repeat(10).to_owned();

        assert_eq!(expected_response, actual_response);
    }

    #[tokio::test]
    async fn test_reading_result_from_response_with_some_escape_bytes() {
        let connection = Connection::from_bytes(&[0xFFu8, 0, 1, 6, 9, 0xFF, 0xFF, 3, 0, 0]);
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).await.unwrap();
        let expected_response = vec![0u8, 1, 6, 9, 0xFF, 3];

        assert_eq!(expected_response, actual_response);
    }

    #[tokio::test]
    async fn test_reading_result_from_response_with_only_escape_bytes() {
        let mut bytes = [0xFFu8, 0].repeat(10);
        bytes.extend([0, 0]);
        let connection = Connection::from_bytes(&bytes);
        let client = Client::new(connection);
        let mut response = Response::new(client);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).await.unwrap();
        let expected_response = [0u8].repeat(10).to_vec();

        assert_eq!(expected_response, actual_response);
    }

    #[tokio::test]
    async fn test_reading_error_from_response() {
        let connection = Connection::from_str("partial_result\0test_error\0\u{1}");
        let client = Client::new(connection);
        let response = Response::new(client);
        let actual_error = response.close().await.err().unwrap();

        assert!(matches!(
            actual_error,
            ConnectionError::CommandFailed(message) if message == "test_error"
        ));
    }

    #[tokio::test]
    #[should_panic]
    async fn test_reading_panics_on_invalid_status_byte() {
        let connection = Connection::from_str("partial_result\0test_error\0\u{2}");
        let client = Client::new(connection);

        let _ = Response::new(client).read(&mut [0u8; 27]).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_reading_panics_on_incomplete_result() {
        let connection = Connection::from_str("partial_result");
        let client = Client::new(connection);

        let _ = Response::new(client).close().await;
    }
}
