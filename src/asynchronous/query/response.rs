use crate::asynchronous::connection::{Authenticated, Connection, ConnectionError, HasConnection};
use crate::asynchronous::query::{Query, QueryFailed};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

/// Response from a [`Query`], produced as an output when you [`execute`] it. Use the [`Read`] trait to read the result.
///
/// The result can be interpreted as UTF-8 characters (and therefore [`read_to_string`]) unless:
/// * [`Query`] result contains binary blobs.
/// * [`Serializer`] is set to different encoding than UTF-8.
///
/// When done with reading, [`close`] the response. Doing so gives you back the [`Query`] instance whose execution
/// created this response.
///
/// # Examples
///
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
/// [`Query`]: crate::Query
/// [`close`]: crate::Query::close
/// [`execute`]: crate::Query::execute
/// [`Serializer`]: crate::serializer
/// [`Read`]: std::io::Read
/// [`read_to_string`]: std::io::Read::read_to_string
#[derive(Debug)]
#[pin_project]
pub struct Response<T, HasInfo>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    #[pin]
    query: Query<T, HasInfo>,
    info_prefix: Option<Vec<u8>>,
    info_complete: bool,
    is_ok: bool,
    result_complete: bool,
}

impl<T, HasInfo> Response<T, HasInfo>
where
    T: AsyncWrite + AsyncRead + Unpin,
    HasInfo: std::marker::Unpin,
{
    pub(crate) fn new(query: Query<T, HasInfo>) -> Self {
        Self {
            query,
            info_prefix: None,
            info_complete: false,
            is_ok: false,
            result_complete: false,
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
    pub async fn close(mut self) -> Result<Query<T, HasInfo>, ConnectionError> {
        let mut buf = [0u8; 4096];

        while !self.result_complete && self.read(&mut buf).await? > 0 {}

        if !self.result_complete {
            panic!("Unexpected end of stream.");
        }

        match self.is_ok {
            true => Ok(self.query),
            false => {
                let info_suffix = if !self.info_complete {
                    let this = Pin::new(&mut self);

                    Some(this.connection().read_string().await?)
                } else {
                    None
                };

                let mut info = String::from_utf8(self.info_prefix.unwrap_or_default())?;

                if let Some(info_suffix) = info_suffix {
                    info.push_str(info_suffix.as_str());
                }

                Err(ConnectionError::QueryFailed(QueryFailed::new(info)))
            }
        }
    }
}

impl<T: AsyncWrite + AsyncRead + Unpin, HasInfo> HasConnection<T> for Response<T, HasInfo> {
    fn connection(self: Pin<&mut Self>) -> Pin<&mut Connection<T, Authenticated>> {
        let this = self.project();

        this.query.connection()
    }
}

impl<T, HasInfo> AsyncRead for Response<T, HasInfo>
where
    T: AsyncWrite + AsyncRead + Unpin,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        if self.result_complete {
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
                        self.result_complete = true;
                        self.is_ok = match buf_unparsed.filled()[..size][position + 1] {
                            0 => true,
                            1 => false,
                            other => panic!("Invalid status byte \"{}\"", other),
                        };
                        if self.is_ok {
                            self.info_complete = true;
                        } else {
                            self.info_prefix =
                                match buf_unparsed.filled()[position + 2..size].iter().position(|&b| b == 0) {
                                    Some(length) => {
                                        self.info_complete = true;
                                        Some(buf_unparsed.filled()[position + 2..position + 2 + length].to_vec())
                                    }
                                    None => Some(buf_unparsed.filled()[position + 2..size].to_vec()),
                                };
                        }
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
    use crate::asynchronous::{client::Client, connection::ConnectionError};

    #[tokio::test]
    async fn test_reading_result_from_response() {
        let connection = Connection::from_str("result\0".to_owned());
        let client = Client::new(connection);

        let query = Query::without_info("1".to_owned(), client);
        let mut response = Response::new(query);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).await.unwrap();
        let expected_response = "result".to_owned();

        assert_eq!(expected_response, actual_response);
    }

    #[tokio::test]
    async fn test_reading_result_from_response_on_multiple_read_calls() {
        let connection = Connection::from_str("result".repeat(10) + "\0");
        let client = Client::new(connection);

        let query = Query::without_info("1".to_owned(), client);
        let mut response = Response::new(query);
        let mut actual_response = String::new();
        response.read_to_string(&mut actual_response).await.unwrap();
        let expected_response = "result".repeat(10).to_owned();

        assert_eq!(expected_response, actual_response);

        response.close().await.expect("Operation must succeed.");
    }

    #[tokio::test]
    async fn test_reading_result_from_response_with_some_escape_bytes() {
        let connection = Connection::from_bytes(&[0xFFu8, 0, 1, 6, 9, 0xFF, 0xFF, 3, 0]);
        let client = Client::new(connection);

        let query = Query::without_info("1".to_owned(), client);
        let mut response = Response::new(query);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).await.unwrap();
        let expected_response = vec![0u8, 1, 6, 9, 0xFF, 3];

        assert_eq!(expected_response, actual_response);

        response.close().await.expect("Operation must succeed.");
    }

    #[tokio::test]
    async fn test_reading_result_from_response_with_only_escape_bytes() {
        let mut bytes = [0xFFu8, 0].repeat(10);
        bytes.extend([0]);
        let connection = Connection::from_bytes(&bytes);
        let client = Client::new(connection);

        let query = Query::without_info("1".to_owned(), client);
        let mut response = Response::new(query);
        let mut actual_response: Vec<u8> = vec![];
        response.read_to_end(&mut actual_response).await.unwrap();
        let expected_response = [0u8].repeat(10).to_vec();

        assert_eq!(expected_response, actual_response);

        response.close().await.expect("Operation must succeed.");
    }

    #[tokio::test]
    async fn test_reading_error_from_response() {
        let expected_error = "Stopped at ., 1/1:\n[XPST0008] Undeclared variable: $x.";
        let connection = Connection::from_str(format!("partial_result\0\u{1}{}\0", expected_error));
        let client = Client::new(connection);

        let query = Query::without_info("1".to_owned(), client);
        let response = Response::new(query);
        let actual_error = response.close().await.err().unwrap();

        assert!(matches!(
            actual_error,
            ConnectionError::QueryFailed(q) if q.raw() == expected_error
        ));
    }

    #[tokio::test]
    async fn test_reading_error_from_response_on_multiple_read_calls() {
        let expected_error = "Stopped at ., 1/1:\n[XPST0008] ".to_owned() + &"error".repeat(5000);
        let connection = Connection::from_str(format!("partial_result\0\u{1}{}\0", expected_error));
        let client = Client::new(connection);
        let query = Query::without_info("1".to_owned(), client);
        let response = Response::new(query);
        let actual_error = response.close().await.err().unwrap();

        assert!(matches!(
            actual_error,
            ConnectionError::QueryFailed(q) if q.raw() == expected_error
        ));
    }

    #[tokio::test]
    #[should_panic]
    async fn test_reading_panics_on_invalid_status_byte() {
        let connection = Connection::from_str("partial_result\0\u{2}test_error\0".to_owned());
        let client = Client::new(connection);
        let query = Query::without_info("1".to_owned(), client);

        let _ = Response::new(query).read(&mut [0u8; 27]).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_reading_panics_on_incomplete_result() {
        let connection = Connection::from_str("partial_result".to_owned());
        let client = Client::new(connection);
        let query = Query::without_info("1".to_owned(), client);

        let _ = Response::new(query).close().await;
    }
}
