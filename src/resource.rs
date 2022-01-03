use std::io::Read;

/// Responsible for converting to a reader. Implementors of this trait are called 'resources'.
pub trait AsResource<'a> {
    type Reader: Read;

    fn into_read(self) -> Self::Reader;
}

impl<'a, T> AsResource<'a> for &'a mut T
where
    T: Read,
{
    type Reader = &'a mut T;

    fn into_read(self) -> Self::Reader {
        self
    }
}

impl<'a> AsResource<'a> for &'a str {
    type Reader = &'a [u8];

    fn into_read(self) -> Self::Reader {
        self.as_bytes()
    }
}
