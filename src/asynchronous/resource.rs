use tokio::io::AsyncRead;

/// Responsible for converting to a reader. Implementors of this trait are called 'resources'.
pub trait AsResource<'a> {
    type Reader: AsyncRead + Unpin;

    fn into_read(self) -> Self::Reader;
}

impl<'a, T> AsResource<'a> for T
where
    T: AsyncRead + Unpin,
{
    type Reader = T;

    fn into_read(self) -> Self::Reader {
        self
    }
}
