use crate::connection::Authenticated;
use crate::resource::AsResource;
use crate::{Connection, DatabaseStream, Result};
use std::net::IpAddr;

/// Writes argument values using a [`Connection`].
///
/// # Example
/// ```
/// # use basex::{ArgumentWriter, ClientError, DatabaseStream, Result};
/// fn write_xquery<T: DatabaseStream>(writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
///     writer.write("data")
/// }
/// ```
/// [`Connection`]: crate::connection::Connection
#[derive(Debug)]
pub struct ArgumentWriter<'a, T: DatabaseStream>(pub &'a mut Connection<T, Authenticated>);

impl<'a, T: DatabaseStream> ArgumentWriter<'a, T> {
    /// Writes bytes from the given reader as the argument's value.
    ///
    /// # Example
    /// ```
    /// # use basex::{ArgumentWriter, ClientError, DatabaseStream, Result};
    /// fn write_xquery<T: DatabaseStream>(writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
    ///     writer.write("data")
    /// }
    /// ```
    pub fn write<'b, R: AsResource<'b>>(&mut self, argument: R) -> Result<()> {
        self.0.send_arg(&mut argument.into_read()).map(|_| ())
    }
}

/// Makes this type able to be interpreted as XQuery argument value.
pub trait ToQueryArgument<'a> {
    /// Writes this value using the given `writer` as an XQuery argument value.
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()>;

    /// The type name of the XQuery representation.
    ///
    /// # Example
    /// ```
    /// use basex::ToQueryArgument;
    /// assert_eq!("xs:string", String::xquery_type());
    /// ```
    fn xquery_type() -> String;
}

impl<'a> ToQueryArgument<'a> for bool {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:boolean".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for u8 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:unsignedByte".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for i8 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:byte".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for u16 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:unsignedShort".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for i16 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:short".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for u32 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:unsignedInt".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for i32 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:int".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for u64 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:unsignedLong".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for i64 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:long".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for f32 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:float".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for f64 {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.to_string().as_str().as_bytes())
    }

    fn xquery_type() -> String {
        "xs:double".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for &'a str {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.as_bytes())
    }

    fn xquery_type() -> String {
        "xs:string".to_owned()
    }
}

impl<'a> ToQueryArgument<'a> for String {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        writer.write(&mut self.as_bytes())
    }

    fn xquery_type() -> String {
        "xs:string".to_owned()
    }
}

impl<'a, 'b, D: ToQueryArgument<'a>> ToQueryArgument<'a> for &'b D {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        (*self).write_xquery(writer)
    }

    fn xquery_type() -> String {
        D::xquery_type()
    }
}

impl<'a, D: ToQueryArgument<'a>> ToQueryArgument<'a> for Option<D> {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        self.as_ref().unwrap().write_xquery(writer)
    }

    fn xquery_type() -> String {
        D::xquery_type()
    }
}

impl<'a> ToQueryArgument<'a> for IpAddr {
    fn write_xquery<T: DatabaseStream>(&self, writer: &mut ArgumentWriter<'_, T>) -> Result<()> {
        self.to_string().write_xquery(writer)
    }

    fn xquery_type() -> String {
        "xs:string".to_owned()
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
    fn test_writing_values_as_query_argument<'a, T: ToQueryArgument<'a>>(
        value: T,
        expected_stream: &str,
        expected_type: &str,
    ) {
        let mut connection = Connection::from_str("");
        let mut writer = ArgumentWriter(&mut connection);
        value.write_xquery(&mut writer).unwrap();
        let actual_stream = connection.into_inner().to_string();

        assert_eq!(expected_stream, actual_stream);
        assert_eq!(expected_type, T::xquery_type());
    }
}
