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
//! println!("{}", value);
//! ```
//!
//! # Conversions
//!
//! This library also supports using the standard `Encodable` and `Decodable`
//! traits with TOML values. This library provides the following conversion
//! capabilities:
//!
//! * `String` => `toml::Value` - via `Parser`
//! * `toml::Value` => `String` - via `Show`
//! * `toml::Value` => rust object - via `Decoder`
//! * rust object => `toml::Value` - via `Encoder`
//!
//! Convenience functions for performing multiple conversions at a time are also
//! provided.
//!
//! [1]: https://github.com/mojombo/toml
//! [2]: https://github.com/BurntSushi/toml-test
//!

#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

extern crate "rustc-serialize" as rustc_serialize;

use std::collections::BTreeMap;
use std::str::FromStr;
use std::string;

pub use parser::{Parser, ParserError};
pub use serialization::{Encoder, encode, encode_str};
pub use serialization::{Decoder, decode, decode_str};
pub use serialization::Error;
pub use serialization::Error::{NeedsKey, NoValue};
pub use serialization::Error::{InvalidMapKeyLocation, InvalidMapKeyType};
pub use serialization::{DecodeError, DecodeErrorKind};
pub use serialization::DecodeErrorKind::{ApplicationError, ExpectedField};
pub use serialization::DecodeErrorKind::{ExpectedMapElement, ExpectedMapKey, NoEnumVariants};
pub use serialization::DecodeErrorKind::{ExpectedType, NilTooLong};

mod parser;
mod show;
mod serialization;
#[cfg(test)]mod test;
/// Representation of a TOML value.
#[derive(PartialEq, Clone)]
#[allow(missing_docs)]
pub enum Value {
    String(string::String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(string::String),
    Array(Array),
    Table(Table),
}

/// Type representing a TOML array, payload of the Value::Array variant
pub type Array = Vec<Value>;

/// Type representing a TOML table, payload of the Value::Table variant
pub type Table = BTreeMap<string::String, Value>;

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
    pub fn as_str<'a>(&'a self) -> Option<&'a str> {
        match *self { Value::String(ref s) => Some(s.as_slice()), _ => None }
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
    pub fn as_datetime<'a>(&'a self) -> Option<&'a str> {
        match *self { Value::Datetime(ref s) => Some(s.as_slice()), _ => None }
    }

    /// Extracts the array value if it is an array.
    pub fn as_slice<'a>(&'a self) -> Option<&'a [Value]> {
        match *self { Value::Array(ref s) => Some(s.as_slice()), _ => None }
    }

    /// Extracts the table value if it is a table.
    pub fn as_table<'a>(&'a self) -> Option<&'a Table> {
        match *self { Value::Table(ref s) => Some(s), _ => None }
    }

    /// Lookups for value at specified path.
    ///
    /// Uses '.' as a path separator.
    ///
    /// Note: arrays have zero-based indexes.
    ///
    /// ```
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
    pub fn lookup<'a>(&'a self, path: &'a str) -> Option<&'a Value> {
        let mut cur_value = self;
        for key in path.split('.') {
            match cur_value {
                &Value::Table(ref hm) => {
                    match hm.get(key) {
                        Some(v) => cur_value = v,
                        None => return None
                    }
                },
                &Value::Array(ref v) => {
                    match key.parse::<uint>() {
                        Some(idx) if idx < v.len() => cur_value = &v[idx],
                        _ => return None
                    }
                },
                _ => return None
            }
        };

        Some(cur_value)
    }
}

impl FromStr for Value {
    fn from_str(s: &str) -> Option<Value> {
        Parser::new(s).parse().map(Value::Table)
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

        let no_bar = value.lookup("test.bar");
        assert!(no_bar.is_none());
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
}
