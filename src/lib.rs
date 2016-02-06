//! A TOML-parsing library
//!
//! This library is an implementation in Rust of a parser for TOML configuration
//! files [1]. It is focused around high quality errors including specific spans
//! and detailed error messages when things go wrong.
//!
//! This implementation currently passes the language agnostic [test suite][2].
//!
//! # Example
//!
//! ```
//! let toml = r#"
//!     [test]
//!     foo = "bar"
//! "#;
//!
//! let value = toml::Parser::new(toml).parse().unwrap();
//! println!("{:?}", value);
//! ```
//!
//! # Conversions
//!
//! This library also supports using the standard `Encodable` and `Decodable`
//! traits with TOML values. This library provides the following conversion
//! capabilities:
//!
//! * `String` => `toml::Value` - via `Parser`
//! * `toml::Value` => `String` - via `Display`
//! * `toml::Value` => rust object - via `Decoder`
//! * rust object => `toml::Value` - via `Encoder`
//!
//! Convenience functions for performing multiple conversions at a time are also
//! provided.
//!
//! [1]: https://github.com/mojombo/toml
//! [2]: https://github.com/BurntSushi/toml-test

#![doc(html_root_url = "http://alexcrichton.com/toml-rs")]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

#[cfg(feature = "rustc-serialize")] extern crate rustc_serialize;
#[cfg(feature = "serde")] extern crate serde;

use std::collections::BTreeMap;
use std::str::FromStr;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::fmt::Error as FmtError;

pub use parser::{Parser, ParserError};

#[cfg(any(feature = "rustc-serialize", feature = "serde"))]
pub use self::encoder::{Encoder, Error, encode, encode_str};
#[cfg(any(feature = "rustc-serialize", feature = "serde"))]
pub use self::decoder::{Decoder, DecodeError, DecodeErrorKind, decode, decode_str};

mod parser;
mod display;
#[cfg(any(feature = "rustc-serialize", feature = "serde"))]
mod encoder;
#[cfg(any(feature = "rustc-serialize", feature = "serde"))]
mod decoder;

/// Error kind for Lookup errors (Value::lookup())
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupErrorKind {
    /// Error if the string for the lookup has a syntax error
    LookupStringSyntaxError,

    /// Key is not found
    KeyNotFound,

    /// Path expects another type than found
    PathTypeFailure,
}

impl LookupErrorKind {

    /// Get the LookupErrorKind as human readable string (not intended as Display replacement)
    pub fn as_str(&self) -> &'static str {
        match self {
            &LookupErrorKind::LookupStringSyntaxError =>
                "Syntax error in lookup string",
            &LookupErrorKind::KeyNotFound => "Key not found",
            &LookupErrorKind::PathTypeFailure => "Path type failure",
        }
    }

}

/// Error type for lookup()
#[derive(Debug)]
pub struct LookupError {
    kind: LookupErrorKind,
    cause: Option<Box<StdError>>,
}

impl LookupError {

    /// Create a new LookupError
    pub fn new(k: LookupErrorKind, c: Option<Box<StdError>>) -> LookupError {
        LookupError {
            kind: k,
            cause: c,
        }
    }

}

impl Display for LookupError {

    fn fmt(&self, fmt: &mut Formatter) -> Result<(), FmtError> {
        try!(write!(fmt, "{}", self.kind.as_str()));
        Ok(())
    }

}

impl StdError for LookupError {

    fn description(&self) -> &str {
        self.kind.clone().as_str().clone()
    }

    fn cause(&self) -> Option<&StdError> {
        self.cause.as_ref().map(|e| &**e)
    }

}

/// newtype for all results from lookup() functionality
pub type LookupResult<T> = Result<T, LookupError>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Key(String),
    Index(usize),
}

/// Representation of a TOML value.
#[derive(PartialEq, Clone, Debug)]
#[allow(missing_docs)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(String),
    Array(Array),
    Table(Table),
}

/// Type representing a TOML array, payload of the Value::Array variant
pub type Array = Vec<Value>;

/// Type representing a TOML table, payload of the Value::Table variant
pub type Table = BTreeMap<String, Value>;

impl Value {
    /// Tests whether this and another value have the same type.
    pub fn same_type(&self, other: &Value) -> bool {
        match (self, other) {
            (&Value::String(..), &Value::String(..)) |
            (&Value::Integer(..), &Value::Integer(..)) |
            (&Value::Float(..), &Value::Float(..)) |
            (&Value::Boolean(..), &Value::Boolean(..)) |
            (&Value::Datetime(..), &Value::Datetime(..)) |
            (&Value::Array(..), &Value::Array(..)) |
            (&Value::Table(..), &Value::Table(..)) => true,

            _ => false,
        }
    }

    /// Returns a human-readable representation of the type of this value.
    pub fn type_str(&self) -> &'static str {
        match *self {
            Value::String(..) => "string",
            Value::Integer(..) => "integer",
            Value::Float(..) => "float",
            Value::Boolean(..) => "boolean",
            Value::Datetime(..) => "datetime",
            Value::Array(..) => "array",
            Value::Table(..) => "table",
        }
    }

    /// Extracts the string of this value if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match *self { Value::String(ref s) => Some(&**s), _ => None }
    }

    /// Extracts the integer value if it is an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match *self { Value::Integer(i) => Some(i), _ => None }
    }

    /// Extracts the float value if it is a float.
    pub fn as_float(&self) -> Option<f64> {
        match *self { Value::Float(f) => Some(f), _ => None }
    }

    /// Extracts the boolean value if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self { Value::Boolean(b) => Some(b), _ => None }
    }

    /// Extracts the datetime value if it is a datetime.
    ///
    /// Note that a parsed TOML value will only contain ISO 8601 dates. An
    /// example date is:
    ///
    /// ```notrust
    /// 1979-05-27T07:32:00Z
    /// ```
    pub fn as_datetime(&self) -> Option<&str> {
        match *self { Value::Datetime(ref s) => Some(&**s), _ => None }
    }

    /// Extracts the array value if it is an array.
    pub fn as_slice(&self) -> Option<&[Value]> {
        match *self { Value::Array(ref s) => Some(&**s), _ => None }
    }

    /// Extracts the table value if it is a table.
    pub fn as_table(&self) -> Option<&Table> {
        match *self { Value::Table(ref s) => Some(s), _ => None }
    }

    /// Lookups for value at specified path.
    ///
    /// Uses '.' as a path separator.
    ///
    /// Note: arrays have zero-based indexes.
    ///
    /// Note: empty path returns self.
    ///
    /// ```
    /// # #![allow(unstable)]
    /// let toml = r#"
    ///      [test]
    ///      foo = "bar"
    ///
    ///      [[values]]
    ///      foo = "baz"
    ///
    ///      [[values]]
    ///      foo = "qux"
    /// "#;
    /// let value: toml::Value = toml.parse().unwrap();
    ///
    /// let foo = value.lookup("test.foo").unwrap();
    /// assert_eq!(foo.as_str().unwrap(), "bar");
    ///
    /// let foo = value.lookup("values.1.foo").unwrap();
    /// assert_eq!(foo.as_str().unwrap(), "qux");
    ///
    /// let no_bar = value.lookup("test.bar");
    /// assert_eq!(no_bar.is_none(), true);
    /// ```
    pub fn lookup<'a>(&'a mut self, path: &str) -> LookupResult<&'a mut Value> {
        self.lookup_mut(path).map(|v| v)
    }

    /// Same as Value::lookup() but returns the value mutable
    pub fn lookup_mut<'a>(&'a mut self, path: &str) -> LookupResult<&'a mut Value> {
        let tokens = Value::tokenize(path);
        if tokens.is_err() {
            return tokens.map(|_| self);
        }
        Value::walk(self, tokens.unwrap())
    }

    /// Set a value inside.
    ///
    /// Returns true if the value was set, false if there was already a value at this place
    pub fn set_by_path(&mut self, path: &str, v: Value) -> LookupResult<bool> {
        let tokens = Value::tokenize(path);
        if tokens.is_err() { // return parser error if any
            return tokens.map(|_| false);
        }
        let tokens = tokens.unwrap();

        let destination = tokens.iter().last();
        if destination.is_none() {
            return Err(LookupError::new(LookupErrorKind::LookupStringSyntaxError, None));
        }
        let destination = destination.unwrap();

        let path_to_dest = tokens[..(tokens.len() - 2)].into(); // N - 1 tokens
        let value = Value::walk(self, path_to_dest); // walk N-1 tokens
        if value.is_err() {
            return value.map(|_| false);
        }
        let mut value = value.unwrap();

        // There is already an value at this place
        if Value::extract(value, destination).is_ok() {
            return Ok(false);
        }

        match destination {
            &Token::Key(ref s) => { // if the destination shall be an map key
                match value {
                    /*
                     * Put it in there if we have a map
                     */
                    &mut Value::Table(ref mut t) => {
                        t.insert(s.clone(), v);
                    }

                    /*
                     * Fail if there is no map here
                     */
                    _ => return Err(LookupError::new(LookupErrorKind::PathTypeFailure, None)),
                }
            },

            &Token::Index(i) => { // if the destination shall be an array
                match value {

                    /*
                     * Put it in there if we have an array
                     */
                    &mut Value::Array(ref mut a) => {
                        a.push(v); // push to the end of the array

                        // if the index is inside the array, we swap-remove the element at this
                        // index
                        if a.len() < i {
                            a.swap_remove(i);
                        }
                    },

                    /*
                     * Fail if there is no array here
                     */
                    _ => return Err(LookupError::new(LookupErrorKind::PathTypeFailure, None)),
                }
            },
        }

        Ok(true)
    }

    fn tokenize(path: &str) -> LookupResult<Vec<Token>> {
        use std::str::FromStr;

        path.split(".")
            .map(|s| {
                usize::from_str(s)
                    .map(Token::Index)
                    .or_else(|_| Ok(Token::Key(String::from(s))))
            })
            .collect()
    }

    fn walk(v: &mut Value, tokens: Vec<Token>) -> LookupResult<&mut Value> {
        use std::vec::IntoIter;

        fn walk_iter<'a>(v: Result<&'a mut Value, LookupError>,
                         i: &mut IntoIter<Token>)
            -> Result<&'a mut Value, LookupError>
        {
            let next = i.next();
            v.and_then(move |value| {
                if let Some(token) = next {
                    walk_iter(Value::extract(value, &token), i)
                } else {
                    Ok(value)
                }
            })
        }

        walk_iter(Ok(v), &mut tokens.into_iter())
    }


    fn extract_from_table<'a>(v: &'a mut Value, s: &String) -> LookupResult<&'a mut Value> {
        match v {
            &mut Value::Table(ref mut t) => {
                t.get_mut(&s[..])
                    .ok_or(LookupError::new(LookupErrorKind::KeyNotFound, None))
            },
            _ => Err(LookupError::new(LookupErrorKind::PathTypeFailure, None)),
        }
    }

    fn extract_from_array(v: &mut Value, i: usize) -> LookupResult<&mut Value> {
        match v {
            &mut Value::Array(ref mut a) => Ok(&mut a[i]),
            _ => Err(LookupError::new(LookupErrorKind::PathTypeFailure, None)),
        }
    }

    fn extract<'a>(v: &'a mut Value, token: &Token) -> LookupResult<&'a mut Value> {
        match token {
            &Token::Key(ref s)  => Value::extract_from_table(v, s),
            &Token::Index(i)    => Value::extract_from_array(v, i),
        }
    }

}

impl FromStr for Value {
    type Err = Vec<ParserError>;
    fn from_str(s: &str) -> Result<Value, Vec<ParserError>> {
        let mut p = Parser::new(s);
        match p.parse().map(Value::Table) {
            Some(n) => Ok(n),
            None => Err(p.errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn lookup_valid() {
        let toml = r#"
              [test]
              foo = "bar"

              [[values]]
              foo = "baz"

              [[values]]
              foo = "qux"
        "#;

        let value: Value = toml.parse().unwrap();

        let test_foo = value.lookup("test.foo").unwrap();
        assert_eq!(test_foo.as_str().unwrap(), "bar");

        let foo1 = value.lookup("values.1.foo").unwrap();
        assert_eq!(foo1.as_str().unwrap(), "qux");

        assert!(value.lookup("test.bar").is_none());
        assert!(value.lookup("test.foo.bar").is_none());
    }

    #[test]
    fn lookup_invalid_index() {
        let toml = r#"
            [[values]]
            foo = "baz"
        "#;

        let value: Value = toml.parse().unwrap();

        let foo = value.lookup("test.foo");
        assert!(foo.is_none());

        let foo = value.lookup("values.100.foo");
        assert!(foo.is_none());

        let foo = value.lookup("values.str.foo");
        assert!(foo.is_none());
    }

    #[test]
    fn lookup_self() {
        let value: Value = r#"foo = "bar""#.parse().unwrap();

        let foo = value.lookup("foo").unwrap();
        assert_eq!(foo.as_str().unwrap(), "bar");

        let foo = value.lookup("").unwrap();
        assert!(foo.as_table().is_some());

        let baz = foo.lookup("foo").unwrap();
        assert_eq!(baz.as_str().unwrap(), "bar");
    }
}
