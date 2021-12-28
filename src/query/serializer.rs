//! [Serialization](https://docs.basex.org/wiki/Serialization) parameters define how XQuery items and XML nodes will be
//! serialized when returned to the client.
//!
//! The official parameters are defined in the
//! [W3C XQuery Serialization 3.1](https://www.w3.org/TR/xslt-xquery-serialization-31) document.
//!
//! # Examples
//!
//! ```
//! use basex::Client;
//! use basex::serializer::Options;
//! # use std::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! // 1.a) Create empty options (server assumes default values)
//! let options = Options::empty();
//!
//! // 1.b) Get options from the server
//! let client = Client::connect("localhost", 1984, "admin", "admin")?;
//! let mut query = client.query("/")?.without_info()?;
//! let mut options = query.options()?;
//!
//! // 2.a) Change options
//! options.set("indent", false);
//! options.set("encoding", "UTF-8");
//!
//! // 2.b) Read options
//! if let Some(Ok(encoding)) = options.get::<&str>("encoding") {
//!     println!("Encoding: {}", encoding);
//! }
//! println!("Indentation: {}", if options.get::<bool>("indent").unwrap()? { "on" } else { "off" });
//!
//! // 3. Save options to the server
//! let client = options.save(query.close()?)?;
//! # Ok(())
//! # }
//! ```
//! Reading result from a [`Query`] would now be affected by the options. In this case the difference as apposed to the
//! default would be that XML nodes are not indented from the beginning of a line.
//!
//! [`Query`]: super::Query
use crate::{Client, DatabaseStream};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::result;
use std::str::FromStr;

type Result<T> = result::Result<T, ParseError>;

/// Error that have occurred when parsing the option's value.
#[derive(Debug)]
pub struct ParseError {
    value: String,
}

impl ParseError {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("expected yes/no, got: {}", self.value))
    }
}

impl Error for ParseError {}

/// Options for query [serializer](https://docs.basex.org/wiki/Serialization).
///
/// # Examples
///
/// ```
/// # use basex::{Client, serializer::Options, serializer::ParseError};
/// # use std::str::FromStr;
/// # fn main() -> std::result::Result<(), std::boxed::Box<dyn std::error::Error>> {
/// // Connect to the server
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// // Create empty options
/// let mut options = Options::empty();
///
/// // Turn off indent option
/// options.set("indent", false);
///
/// // Change encoding option
/// options.set("encoding", "UTF-8");
///
/// // Check the options we just set
/// let indent: bool = !options.get("indent").unwrap()?;
/// assert!(indent);
/// let encoding: &str = options.get("encoding").unwrap()?;
/// assert_eq!("UTF-8", encoding);
///
/// // Save the options to database
/// let client = options.save(client)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    options: BTreeMap<String, String>,
}

impl Options {
    fn new(options: BTreeMap<String, String>) -> Self {
        Self { options }
    }

    /// Creates empty options. The server sets its own defaults if this is saved.
    pub fn empty() -> Self {
        Self::new(BTreeMap::new())
    }

    /// Gets mutable reference to an attribute if it exists.
    pub fn get<'a, T: FromAttribute<'a>>(&'a self, key: &str) -> Option<Result<T>> {
        self.options.get(key).map(|v| T::from_str(v))
    }

    /// Inserts new attribute value.
    pub fn set(&mut self, key: &str, value: impl ToAttribute) {
        self.options.insert(key.to_owned(), value.as_str().to_owned());
    }

    /// Saves the options to the server serializer for current session.
    pub fn save<T: DatabaseStream>(&self, client: Client<T>) -> crate::Result<Client<T>> {
        let (client, _) = client
            .execute(&format!("SET SERIALIZER {}", self.to_string()))?
            .close()?;
        Ok(client)
    }
}

impl ToString for Options {
    fn to_string(&self) -> String {
        let mut str = String::new();
        for (key, value) in self.options.iter() {
            if !str.is_empty() {
                str.push(',');
            }
            str.push_str(key);
            str.push('=');
            str.push_str(value);
        }
        str
    }
}

impl FromStr for Options {
    type Err = ParseError;

    /// Reads the options as comma separated list of key=value pairs.
    ///
    /// Cannot result in `Err` state. The logic is infallible, read a key and stop at "`=`" then switch to value until
    /// "`,`", repeat until the end.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use std::str::FromStr;
    /// # use basex::serializer::Options;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// // Create options from string (not loaded from the database)
    /// let mut options = Options::from_str("encoding=US-ASCII,indent=yes")?;
    /// let indent: bool = options.get("indent").unwrap()?;
    /// assert!(indent);
    /// let encoding: &str = options.get("encoding").unwrap()?;
    /// assert_eq!("US-ASCII", encoding);
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(s: &str) -> Result<Self> {
        let mut options: BTreeMap<String, String> = BTreeMap::new();
        let mut tuple = (String::new(), String::new());
        let mut key_complete = false;
        for x in s.chars() {
            if x == '=' {
                key_complete = true;
                continue;
            }
            if x == ',' {
                options.insert(tuple.0.to_owned(), tuple.1.to_owned());
                tuple.0.clear();
                tuple.1.clear();
                key_complete = false;
                continue;
            }
            if key_complete {
                tuple.1.push(x);
            } else {
                tuple.0.push(x);
            }
        }
        if !tuple.0.is_empty() {
            options.insert(tuple.0.to_owned(), tuple.1.to_owned());
        }

        Ok(Options::new(options))
    }
}

/// Creates the value from attribute string representation.
pub trait FromAttribute<'a>: Sized {
    /// Returns the value from string.
    fn from_str(s: &'a str) -> Result<Self>;
}

impl FromAttribute<'_> for bool {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(ParseError::new(s)),
        }
    }
}

impl<'a> FromAttribute<'a> for &'a str {
    fn from_str(s: &'a str) -> Result<Self> {
        Ok(s)
    }
}

/// Converts the value to attribute string representation.
pub trait ToAttribute {
    /// Returns this value as string.
    fn as_str(&self) -> &str;
}

impl ToAttribute for bool {
    fn as_str(&self) -> &str {
        if *self {
            "yes"
        } else {
            "no"
        }
    }
}

impl ToAttribute for &str {
    fn as_str(&self) -> &str {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloning_options_produces_same_options() -> Result<()> {
        let expected_options = Options::from_str("encoding=US-ASCII,indent=yes")?;
        let actual_options = expected_options.clone();
        assert_eq!(expected_options, actual_options);
        Ok(())
    }

    #[test]
    fn test_true_attribute_as_bool_is_true() {
        let test: bool = FromAttribute::from_str(true.as_str()).unwrap();
        assert!(test);
    }

    #[test]
    fn test_false_attribute_as_bool_is_false() {
        let test: bool = FromAttribute::from_str(false.as_str()).unwrap();
        assert!(!test);
    }

    #[test]
    #[should_panic]
    fn test_non_boolean_str_panics_as_bool() {
        let _: bool = FromAttribute::from_str("test").unwrap();
    }

    #[test]
    fn test_parse_error_formats_as_debug() {
        format!("{:?}", ParseError::new("test"));
    }

    #[test]
    fn test_parse_error_formats_as_empty() {
        format!("{}", ParseError::new("test"));
    }

    #[test]
    fn test_options_empty_has_no_attributes() {
        format!("{:?}", Options::empty());
    }

    #[test]
    fn test_options_formats_as_debug() {
        format!("{:?}", Options::empty());
    }

    #[test]
    fn test_attributes_can_be_inserted_into_options() {
        let mut options = Options::from_str("").unwrap();
        options.set("indent", false);
        options.set("encoding", "UTF-8");
        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
    }

    #[test]
    fn test_attributes_can_be_read_from_options() -> Result<()> {
        let options = Options::from_str("encoding=UTF-8,indent=yes")?;
        assert_eq!(options.get::<bool>("indent").unwrap()?, true);
        assert_eq!(options.get::<&str>("encoding").unwrap()?, "UTF-8");
        Ok(())
    }

    #[test]
    fn test_changing_value_changes_options() -> Result<()> {
        let mut options = Options::from_str("encoding=US-ASCII,indent=yes")?;

        let indent: bool = options.get("indent").unwrap()?;
        assert!(indent);
        options.set("indent", false);
        let indent: bool = options.get("indent").unwrap()?;
        assert!(!indent);

        let encoding: &str = options.get("encoding").unwrap()?;
        assert_eq!("US-ASCII", encoding);
        options.set("encoding", "UTF-8");
        let encoding: &str = options.get("encoding").unwrap()?;
        assert_eq!("UTF-8", encoding);

        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
        Ok(())
    }
}
