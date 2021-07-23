use super::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{Read, Write};

pub(crate) struct MockStream {
    buffer: Rc<RefCell<Vec<u8>>>,
    response: String,
}

impl MockStream {
    pub(crate) fn new(response: String) -> Self {
        Self { buffer: Rc::new(RefCell::new(vec![])), response }
    }
}

impl ToString for MockStream {
    fn to_string(&self) -> String {
        String::from_utf8(self.buffer.borrow().clone()).unwrap()
    }
}

impl Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.response.as_bytes().len();
        (&mut *buf).write_all(self.response.as_bytes())?;
        (&mut *buf).write(&[0 as u8])?;
        Ok(size)
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
        Ok(MockStream {
            buffer: Rc::clone(&self.buffer),
            response: self.response.clone()
        })
    }
}
