use pin_project::pin_project;
use std::cmp::min;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

/// Wraps a reader and escapes all bytes with special meaning as defined by
/// [conventions](https://docs.basex.org/wiki/Server_Protocol#Conventions).
///
/// All bytes that have any special meaning get prefixed by a single `0xFF` byte.
///
/// # Examples
/// ## Input
/// `[0, 1, 2, 3, 4, 0xFF]`
/// ## Output
/// `[0xFF, 0, 1, 2, 3, 4, 0xFF, 0xFF]`
#[pin_project]
pub(crate) struct EscapeReader<'a, R>
where
    R: AsyncRead + Unpin,
{
    #[pin]
    inner: &'a mut R,
    #[pin]
    accumulator: Vec<u8>,
}

impl<'a, R> EscapeReader<'a, R>
where
    R: AsyncRead + Unpin,
{
    pub(crate) fn new(inner: &'a mut R) -> Self {
        Self {
            inner,
            accumulator: vec![],
        }
    }
}

impl<R> AsyncRead for EscapeReader<'_, R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let accumulator_length = min(buf.capacity(), self.accumulator.len());
        let mut this = self.project();

        for _ in 0..accumulator_length {
            buf.put_slice(&[this.accumulator.pop().unwrap()]);
        }

        let length = buf.filled().len();

        match this.inner.poll_read(cx, buf)? {
            Poll::Pending => Poll::Pending,
            Poll::Ready(..) => {
                let stream_length = buf.filled().len() - length;
                let size = accumulator_length + stream_length;
                let escape_chars_count = buf.filled()[accumulator_length..size]
                    .iter()
                    .filter(|b| **b == 0 || **b == 0xFF)
                    .count();
                let escaped_size = size + escape_chars_count;
                let mut shift = escape_chars_count;
                let mut next_skip = false;

                let extension = min(buf.remaining(), escape_chars_count);
                buf.put_slice(&[0].repeat(extension));

                for i in (accumulator_length..escaped_size).rev() {
                    if next_skip {
                        next_skip = false;
                        continue;
                    }
                    if i >= buf.capacity() {
                        this.accumulator.push(buf.filled()[i - shift]);
                    } else {
                        buf.initialized_mut()[i] = buf.filled()[i - shift];
                    }

                    if buf.filled()[i - shift] == 0xFF || buf.filled()[i - shift] == 0 {
                        if i <= buf.capacity() {
                            buf.initialized_mut()[i - 1] = 0xFF;
                        } else {
                            this.accumulator.push(0xFF);
                        }
                        shift -= 1;
                        next_skip = true;
                    }
                }

                Poll::Ready(Ok(()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::empty;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_escaping_without_escape_bytes_leaves_buffer_intact() {
        let expected_bytes = [1u8, 2, 3, 4];
        let mut slice = &expected_bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes.to_vec(), actual_bytes);
    }

    #[tokio::test]
    async fn test_escaping_with_escape_bytes() {
        let bytes = [1u8, 0, 9, 0xFF, 6];
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes = vec![1u8, 0xFF, 0, 9, 0xFF, 0xFF, 6];
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[tokio::test]
    async fn test_escaping_only_escape_bytes() {
        let bytes = [0u8].repeat(4);
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes = [0xFF, 0u8].repeat(4);
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[tokio::test]
    async fn test_escaping_only_escape_bytes_on_multiple_reading() {
        let bytes = [0u8].repeat(20);
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes = [0xFF, 0u8].repeat(20);
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[tokio::test]
    async fn test_escaping_from_empty_reader_does_nothing() {
        let mut bytes = empty();
        let mut escaped = EscapeReader::new(&mut bytes);

        let expected_bytes: Vec<u8> = vec![];
        let mut actual_bytes = vec![];
        escaped.read_to_end(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }

    #[tokio::test]
    async fn test_escaping_into_empty_buffer_does_nothing() {
        let bytes = [1u8];
        let mut slice = &bytes[..];
        let mut escaped = EscapeReader::new(&mut slice);

        let expected_bytes: [u8; 0] = [];
        let mut actual_bytes: [u8; 0] = [];
        escaped.read(&mut actual_bytes).await.unwrap();

        assert_eq!(expected_bytes, actual_bytes);
    }
}
