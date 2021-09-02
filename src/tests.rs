use super::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{Read, Write, copy};
use circbuf::CircBuf;

pub(crate) struct MockStream {
    buffer: Rc<RefCell<Vec<u8>>>,
    response: CircBuf,
}

impl MockStream {
    pub(crate) fn new(response: String) -> Self {
        let mut buffer = CircBuf::with_capacity(response.len() + 1).unwrap();
        buffer.write_all(response.as_bytes()).unwrap();
        buffer.write(&[0]).unwrap();

        Self { buffer: Rc::new(RefCell::new(vec![])), response: buffer }
    }
}

impl ToString for MockStream {
    fn to_string(&self) -> String {
        String::from_utf8(self.buffer.borrow().clone()).unwrap()
    }
}

impl Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.response.read(buf)
    }
}

impl Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bytes_written = buf.len();
        self.buffer.borrow_mut().extend(buf);
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unimplemented!()
    }
}

impl DatabaseStream for MockStream {
    fn try_clone(&mut self) -> Result<Self> {
        let mut cloned_buff = CircBuf::with_capacity(self.response.len()).unwrap();
        copy(&mut self.response, &mut cloned_buff)?;

        Ok(MockStream {
            buffer: Rc::clone(&self.buffer),
            response: cloned_buff,
        })
    }
}

pub(crate) struct FailingStream;

impl Read for FailingStream {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, ""))
    }
}

impl Write for FailingStream {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, ""))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        unimplemented!()
    }
}

impl DatabaseStream for FailingStream {
    fn try_clone(&mut self) -> Result<Self> {
        unimplemented!()
    }
}
