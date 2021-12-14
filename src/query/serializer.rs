use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::result;
use crate::{Client, DatabaseStream, Result};

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

impl Error for ParseError {
}

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
/// let mut indent = options.get_mut("indent").unwrap().as_bool()?;
/// assert!(indent.enabled());
/// indent.disable();
/// assert!(!indent.enabled());
///
/// // Change encoding option
/// let mut encoding = options.get_mut("encoding").unwrap().as_text()?;
/// assert_eq!("US-ASCII", encoding.as_str());
/// encoding.change("UTF-8");
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
#[derive(Debug)]
pub struct Options {
    options: BTreeMap<String, Attribute>,
}

impl Options {
    fn new(options: BTreeMap<String, Attribute>) -> Self {
        Self {
            options,
        }
    }

    /// Gets mutable reference to an attribute if it exists.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Attribute> {
        self.options.get_mut(key)
    }

    /// Inserts new attribute value.
    pub fn insert(&mut self, key: &str, value: Attribute) -> &mut Attribute {
        self.options.insert(key.to_owned(), value);
        self.get_mut(key).unwrap()
    }

    /// Saves the options to the database for current session.
    pub fn save<T: DatabaseStream>(&self, client: Client<T>) -> Result<Client<T>> {
        let (client, _) = client.execute(&format!("SET SERIALIZER {}", self.to_string()))?
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
            str.push_str( &value.to_string());
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
                options.insert(tuple.0.to_owned(), Attribute::new(&tuple.1));
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
            options.insert(tuple.0.to_owned(), Attribute::new(&tuple.1));
        }

        Ok(Options::new(options))
    }
}

/// Attribute of the serializer.
#[derive(Debug)]
pub struct Attribute {
    inner: String,
}

/// [`Attribute`] represented as a boolean.
///
/// [`Attribute`]: self::Attribute
#[derive(Debug)]
pub struct BooleanAttribute<'a>(&'a mut Attribute);

/// [`Attribute`] represented as a textual.
///
/// [`Attribute`]: self::Attribute
#[derive(Debug)]
pub struct TextualAttribute<'a>(&'a mut Attribute);

impl Attribute {
    /// Creates new attribute with inner value.
    pub fn new(inner: &str) -> Self {
        Self {
            inner: inner.to_owned(),
        }
    }

    /// Wraps this attribute as textual.
    pub fn as_text(&mut self) -> result::Result<TextualAttribute, ParseError> {
        Ok(TextualAttribute(self))
    }

    /// Wraps this attribute as boolean.
    pub fn as_bool(&mut self) -> result::Result<BooleanAttribute, ParseError> {
        if self.inner != "yes" && self.inner != "no" {
            return Err(ParseError::new(&self.inner));
        }
        Ok(BooleanAttribute(self))
    }
}

impl TextualAttribute<'_> {
    pub fn as_str(&self) -> &str {
        self.0.inner.as_str()
    }

    pub fn change(&mut self, value: &str) {
        self.0.inner = value.to_owned();
    }
}

impl BooleanAttribute<'_> {
    pub fn yes() -> Attribute {
        Attribute::new("yes")
    }

    pub fn no() -> Attribute {
        Attribute::new("no")
    }

    pub fn enable(&mut self) {
        self.0.inner = "yes".to_owned();
    }

    pub fn disable(&mut self) {
        self.0.inner = "no".to_owned();
    }

    pub fn enabled(&self) -> bool {
        match self.0.inner.as_str() {
            "yes" => true,
            "no" => false,
            other => panic!("Expected yes/no, got: {}", other),
        }
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
    fn test_enable_creates_enabled_attribute() {
        let mut attr = BooleanAttribute::no();
        let mut attr = attr.as_bool().unwrap();
        attr.enable();
        assert!(attr.enabled());
    }

    #[test]
    fn test_disable_creates_disabled_attribute() {
        let mut attr = BooleanAttribute::yes();
        let mut attr = attr.as_bool().unwrap();
        attr.disable();
        assert!(!attr.enabled());
    }

    #[test]
    fn test_yes_creates_enabled_attribute() {
        assert!(BooleanAttribute::yes().as_bool().unwrap().enabled());
    }

    #[test]
    fn test_no_creates_disabled_attribute() {
        assert!(!BooleanAttribute::no().as_bool().unwrap().enabled());
    }

    #[test]
    #[should_panic]
    fn test_non_boolean_panics_enabled() {
        BooleanAttribute(&mut Attribute::new("test")).enabled();
    }

    #[test]
    fn test_non_boolean_fails_as_bool() {
        Attribute::new("test").as_bool().expect_err("Parsing must fail");
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
        format!("{:?}", Attribute::new(""));
    }

    #[test]
    fn test_textual_attribute_formats_as_debug() {
        format!("{:?}", Attribute::new("").as_text().unwrap());
    }

    #[test]
    fn test_boolean_attribute_formats_as_debug() {
        format!("{:?}", BooleanAttribute::yes().as_bool().unwrap());
    }

    #[test]
    fn test_inserting_attributes_into_options() {
        let mut options = Options::from_str("").unwrap();
        options.insert("indent", BooleanAttribute::no());
        options.insert("encoding", Attribute::new("UTF-8"));
        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
    }

    #[test]
    fn test_default_option() {
        let mut options = Options::from_str("").unwrap();
        options.insert("indent", BooleanAttribute::no());
        options.insert("encoding", Attribute::new("UTF-8"));
        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
    }

    #[test]
    fn test_changing_value_changes_options() -> result::Result<(), ParseError> {
        let mut options = Options::from_str("encoding=US-ASCII,indent=yes")?;

        let mut indent = options.get_mut("indent").unwrap().as_bool()?;
        assert!(indent.enabled());
        indent.disable();
        assert!(!indent.enabled());

        let mut encoding = options.get_mut("encoding").unwrap().as_text()?;
        assert_eq!("US-ASCII", encoding.as_str());
        encoding.change("UTF-8");
        assert_eq!("UTF-8", encoding.as_str());

        assert_eq!("encoding=UTF-8,indent=no", &options.to_string());
        Ok(())
    }
}
