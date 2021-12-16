use crate::{Client, DatabaseStream, Result};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::result;
use std::str::FromStr;

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
        f.write_str(&format!("expected boolean option, got: {}", self.value))
    }
}

impl Error for ParseError {}

/// Options for query [serializer](https://docs.basex.org/wiki/Serialization).
///
/// # Example
///
/// ```
/// # use basex::{Client, serializer::Options, serializer::ParseError};
/// # use std::str::FromStr;
/// # fn main() -> std::result::Result<(), std::boxed::Box<dyn std::error::Error>> {
/// // Connect to the server
/// let mut client = Client::connect("localhost", 1984, "admin", "admin")?;
///
/// // Create options from string (not loaded from the database)
/// let mut options = Options::from_str("encoding=US-ASCII,indent=yes")?;
///
/// // Change indent option
/// let indent = options.get("indent").unwrap();
/// assert!(indent.as_bool()?);
/// let indent = options.set("indent", false);
/// assert!(!indent.as_bool()?);
///
/// // Change encoding option
/// let encoding = options.get("encoding").unwrap();
/// assert_eq!("US-ASCII", encoding.as_str());
/// let encoding = options.set("encoding", "UTF-8");
/// assert_eq!("UTF-8", encoding.as_str());
///
/// // Final state
/// assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
///
/// // Save the state of Options to database
/// let client = options.save(client)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    options: BTreeMap<String, Attribute>,
}

impl Options {
    fn new(options: BTreeMap<String, Attribute>) -> Self {
        Self { options }
    }

    /// Gets mutable reference to an attribute if it exists.
    pub fn get(&self, key: &str) -> Option<&Attribute> {
        self.options.get(key)
    }

    /// Inserts new attribute value.
    pub fn set(&mut self, key: &str, value: impl ToAttribute) -> &Attribute {
        self.options.insert(key.to_owned(), value.to_attribute());
        self.get(key).unwrap()
    }

    /// Saves the options to the server serializer for current session.
    pub fn save<T: DatabaseStream>(&self, client: Client<T>) -> Result<Client<T>> {
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
            str.push_str(&value.to_string());
        }
        str
    }
}

impl FromStr for Options {
    type Err = ParseError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let mut options: BTreeMap<String, Attribute> = BTreeMap::new();
        let mut tuple = (String::new(), String::new());
        let mut key_complete = false;
        for x in s.chars() {
            if x == '=' {
                key_complete = true;
                continue;
            }
            if x == ',' {
                options.insert(tuple.0.to_owned(), Attribute::from_str(&tuple.1)?);
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
            options.insert(tuple.0.to_owned(), Attribute::from_str(&tuple.1)?);
        }

        Ok(Options::new(options))
    }
}

pub trait ToAttribute {
    fn to_attribute(&self) -> Attribute;
}

impl ToAttribute for bool {
    fn to_attribute(&self) -> Attribute {
        Attribute::from_str(if *self { "yes" } else { "no" }).unwrap()
    }
}

impl ToAttribute for &str {
    fn to_attribute(&self) -> Attribute {
        Attribute::from_str(self).unwrap()
    }
}

/// Attribute of the serializer.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    inner: String,
}

impl Attribute {
    /// Returns this attribute as str.
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Returns this attribute as boolean.
    pub fn as_bool(&self) -> result::Result<bool, ParseError> {
        match self.inner.as_str() {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(ParseError::new(&self.inner)),
        }
    }
}

impl FromStr for Attribute {
    type Err = ParseError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        Ok(Self { inner: s.to_owned() })
    }
}

impl ToString for Attribute {
    fn to_string(&self) -> String {
        self.inner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloning_options_produces_same_options() -> result::Result<(), ParseError> {
        let expected_options = Options::from_str("encoding=US-ASCII,indent=yes")?;
        let actual_options = expected_options.clone();
        assert_eq!(expected_options, actual_options);
        Ok(())
    }

    #[test]
    fn test_true_attribute_as_bool_is_true() {
        assert!(true.to_attribute().as_bool().unwrap());
    }

    #[test]
    fn test_false_attribute_as_bool_is_false() {
        assert!(!false.to_attribute().as_bool().unwrap());
    }

    #[test]
    fn test_non_boolean_fails_as_bool() {
        Attribute::from_str("test")
            .unwrap()
            .as_bool()
            .expect_err("Parsing must fail");
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
    fn test_options_formats_as_debug() {
        format!("{:?}", Options::new(BTreeMap::new()));
    }

    #[test]
    fn test_attribute_formats_as_debug() {
        format!("{:?}", Attribute::from_str("").unwrap());
    }

    #[test]
    fn test_attributes_can_be_inserted_into_options() {
        let mut options = Options::from_str("").unwrap();
        options.set("indent", false);
        options.set("encoding", "UTF-8");
        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
    }

    #[test]
    fn test_attributes_can_be_read_from_options() -> result::Result<(), ParseError> {
        let options = Options::from_str("encoding=UTF-8,indent=yes")?;
        assert_eq!(*options.get("indent").unwrap(), true.to_attribute());
        assert_eq!(*options.get("encoding").unwrap(), Attribute::from_str("UTF-8").unwrap());
        Ok(())
    }

    #[test]
    fn test_changing_value_changes_options() -> result::Result<(), ParseError> {
        let mut options = Options::from_str("encoding=US-ASCII,indent=yes")?;

        let indent = options.get("indent").unwrap();
        assert!(indent.as_bool()?);
        let indent = options.set("indent", false);
        assert!(!indent.as_bool()?);

        let encoding = options.get("encoding").unwrap();
        assert_eq!("US-ASCII", encoding.as_str());
        let encoding = options.set("encoding", "UTF-8");
        assert_eq!("UTF-8", encoding.as_str());

        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
        Ok(())
    }
}
