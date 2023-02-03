use crate::asynchronous::connection::Connection;
use crate::asynchronous::connection::{Authenticated, ConnectionError};
use crate::asynchronous::resource::AsResource;
use async_trait::async_trait;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Result<T> = std::result::Result<T, ConnectionError>;

/// Writes argument values using a [`Connection`].
///
/// # Examples
///
/// ```
/// # use basex::{ArgumentWriter, ClientError, Stream, Result};
/// fn write_xquery<T: Stream>(mut writer: ArgumentWriter<'_, T>) -> Result<()> {
///     writer.write("data")
/// }
/// ```
/// [`Connection`]: crate::connection::Connection
#[derive(Debug)]
pub struct ArgumentWriter<'a, T: AsyncWriteExt + AsyncReadExt + Unpin>(pub &'a mut Connection<T, Authenticated>);

impl<'a, T: AsyncWriteExt + AsyncReadExt + Unpin> ArgumentWriter<'a, T> {
    /// Writes bytes from the given reader as the argument's value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use basex::{ArgumentWriter, ClientError, Stream, Result};
    /// fn write_xquery<T: Stream>(mut writer: ArgumentWriter<'_, T>) ->Result<()> {
    ///     writer.write("data")
    /// }
    /// ```
    pub async fn write<'b, R: AsResource<'b> + Unpin>(&mut self, argument: R) -> Result<()> {
        self.0.send_arg(&mut argument.into_read()).await.map(|_| ())
    }
}

/// Makes this type able to be interpreted as XQuery argument value.
pub trait ToQueryArgument<'a> {
    /// Writes this value using the given `writer` as an XQuery argument value.
    fn write_xquery<'f, T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send>(&'f self, writer: ArgumentWriter<'f, T>) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>>;

    /// The type name of the XQuery representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use basex::ToQueryArgument;
    /// assert_eq!("xs:string", String::xquery_type());
    /// ```
    fn xquery_type() -> String;
}

/// Macro dedicated to implement given type using its [`to_string`] method to encode.
///
/// [`to_string`]: std::string::ToString::to_string
macro_rules! query_argument_using_to_string {
    ($($t:ty as $name:expr),*) => {
        $(
        impl<'a> ToQueryArgument<'a> for $t {
            fn write_xquery<'f, T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send>(&'f self, mut writer: ArgumentWriter<'f, T>) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>> {
                let data = self.to_string();

                Box::pin(async move {
                    writer.write(&mut data.as_bytes()).await
                })
            }

            fn xquery_type() -> String {
                $name.to_owned()
            }
        })*
    }
}

query_argument_using_to_string![
    bool as "xs:boolean",
    u8 as "xs:unsignedByte",
    i8 as "xs:byte",
    u16 as "xs:unsignedShort",
    i16 as "xs:short",
    u32 as "xs:unsignedInt",
    i32 as "xs:int",
    u64 as "xs:unsignedLong",
    i64 as "xs:long",
    f32 as "xs:float",
    f64 as "xs:double",
    IpAddr as "xs:string"
];

impl<'a> ToQueryArgument<'a> for &'a str {
    fn write_xquery<'f, T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send>(&'f self, mut writer: ArgumentWriter<'f, T>) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>> {
        let mut data = self.as_bytes().clone();

        Box::pin(async move {
            writer.write(&mut data).await
        })
    }

    fn xquery_type() -> String {
        "xs:string".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for String {
    fn write_xquery<'f, T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send>(&'f self, mut writer: ArgumentWriter<'f, T>) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>> {
        let mut data = self.as_bytes().clone();

        Box::pin(async move {
            writer.write(&mut data).await
        })
    }

    fn xquery_type() -> String {
        "xs:string".to_owned()
    }
}

impl<'a, 'b, D: ToQueryArgument<'a> + Sync> ToQueryArgument<'a> for &'b D {
    fn write_xquery<'f, T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send>(&'f self, writer: ArgumentWriter<'f, T>) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>> {
        Box::pin(async move {
            (*self).write_xquery(writer).await
        })
    }

    fn xquery_type() -> String {
        D::xquery_type()
    }
}

impl<'a, D: ToQueryArgument<'a> + Sync> ToQueryArgument<'a> for Option<D> {
    fn write_xquery<'f, T: AsyncWriteExt + AsyncReadExt + Unpin + Sync + Send>(&'f self, writer: ArgumentWriter<'f, T>) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>> {
        let this = self.clone();

        Box::pin(async move {
            match this.as_ref() {
                Some(data) => data.write_xquery(writer),
                None => "".write_xquery(writer),
            }.await
        })
    }

    fn xquery_type() -> String {
        D::xquery_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(IpAddr::V4("125.0.0.1".parse().unwrap()), "125.0.0.1\0", "xs:string")]
    #[test_case("test", "test\0", "xs:string")]
    #[test_case("test".to_owned(), "test\0", "xs:string")]
    #[test_case(5u8, "5\0", "xs:unsignedByte")]
    #[test_case(5u16, "5\0", "xs:unsignedShort")]
    #[test_case(5u32, "5\0", "xs:unsignedInt")]
    #[test_case(5u64, "5\0", "xs:unsignedLong")]
    #[test_case(5i8, "5\0", "xs:byte")]
    #[test_case(5i16, "5\0", "xs:short")]
    #[test_case(5i32, "5\0", "xs:int")]
    #[test_case(5i64, "5\0", "xs:long")]
    #[test_case(true, "true\0", "xs:boolean")]
    #[test_case(5.5f32, "5.5\0", "xs:float")]
    #[test_case(5.5f64, "5.5\0", "xs:double")]
    #[test_case(&5.2f64, "5.2\0", "xs:double")]
    #[test_case(Some(true), "true\0", "xs:boolean")]
    #[test_case(Option::<bool>::None, "\0", "xs:boolean")]
    #[tokio::test]
    async fn test_writing_values_as_query_argument<'a, T: ToQueryArgument<'a>>(
        value: T,
        expected_stream: &str,
        expected_type: &str,
    ) {
        let mut connection = Connection::from_str("");
        value.write_xquery(ArgumentWriter(&mut connection)).await.unwrap();
        let actual_stream = connection.into_inner().to_string();

        assert_eq!(expected_stream, actual_stream);
        assert_eq!(expected_type, T::xquery_type());
    }
}
