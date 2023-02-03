use circbuf::CircBuf;
use pin_project::pin_project;
use std::io::Error;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Debug)]
#[pin_project]
pub(crate) struct MockStream {
    buffer: Arc<Mutex<Vec<u8>>>,
    #[pin]
    response: CircBuf,
}

impl MockStream {
    pub(crate) fn from_bytes(response: &[u8]) -> Self {
        let mut buffer = CircBuf::with_capacity(response.len() + 1).unwrap();
        buffer.write_all(response).unwrap();
        buffer.write(&[0]).unwrap();

        Self {
            buffer: Arc::new(Mutex::new(vec![])),
            response: buffer,
        }
    }

    pub(crate) fn new(response: String) -> Self {
        Self::from_bytes(response.as_bytes())
    }
}

impl ToString for MockStream {
    fn to_string(&self) -> String {
        String::from_utf8(self.buffer.lock().unwrap().clone()).unwrap()
    }
}

impl AsyncRead for MockStream {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        if buf.capacity() - buf.filled().len() > 0 {
            buf.initialize_unfilled();
            let filled = buf.filled().len();

            let size = self.project().response.read(&mut buf.initialized_mut()[filled..])?;
            buf.set_filled(filled + size);
        }

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockStream {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let bytes_written = buf.len();
        self.buffer.lock().unwrap().extend(buf);
        Poll::Ready(Ok(bytes_written))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Clone for MockStream {
    fn clone(&self) -> Self {
        let mut cloned_buff = CircBuf::with_capacity(self.response.len()).unwrap();
        io::copy(&mut self.response.get_bytes()[0], &mut cloned_buff).unwrap();

        MockStream {
            buffer: Arc::clone(&self.buffer),
            response: cloned_buff,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FailingStream;

impl AsyncRead for FailingStream {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::new(std::io::ErrorKind::Other, "")))
    }
}

impl AsyncWrite for FailingStream {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &[u8]) -> Poll<Result<usize, Error>> {
        Poll::Ready(Err(io::Error::new(std::io::ErrorKind::Other, "")))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        unimplemented!()
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        unimplemented!()
    }
}
