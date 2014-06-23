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

#![crate_type = "lib"]
#![feature(macro_rules)]
#![deny(warnings, missing_doc)]
#![allow(visible_private_types)]

extern crate serialize;

use std::collections::HashMap;
use std::from_str::FromStr;

pub use parser::{Parser, Error};
pub use serialization::{Encoder, encode, encode_str};
pub use serialization::{Decoder, decode, decode_str};
pub use serialization::{Error, NeedsKey, NoValue};
pub use serialization::{InvalidMapKeyLocation, InvalidMapKeyType};

mod parser;
mod show;
mod serialization;
#[cfg(test)] mod test;

/// Representation of a TOML value.
#[deriving(PartialEq, Clone)]
#[allow(missing_doc)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(String),
    Array(Array),
    Table(Table),
}

pub type Array = Vec<Value>;
pub type Table = HashMap<String, Value>;

impl Value {
    /// Tests whether this and another value have the same type.
    pub fn same_type(&self, other: &Value) -> bool {
        match (self, other) {
            (&String(..), &String(..)) |
            (&Integer(..), &Integer(..)) |
            (&Float(..), &Float(..)) |
            (&Boolean(..), &Boolean(..)) |
            (&Datetime(..), &Datetime(..)) |
            (&Array(..), &Array(..)) |
            (&Table(..), &Table(..)) => true,

            _ => false,
        }
    }

    /// Returns a human-readable representation of the type of this value.
    pub fn type_str(&self) -> &'static str {
        match *self {
            String(..) => "string",
            Integer(..) => "integer",
            Float(..) => "float",
            Boolean(..) => "boolean",
            Datetime(..) => "datetime",
            Array(..) => "array",
            Table(..) => "table",
        }
    }

    /// Extracts the string of this value if it is a string.
    pub fn as_str<'a>(&'a self) -> Option<&'a str> {
        match *self { String(ref s) => Some(s.as_slice()), _ => None }
    }

    /// Extracts the integer value if it is an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match *self { Integer(i) => Some(i), _ => None }
    }

    /// Extracts the float value if it is a float.
    pub fn as_float(&self) -> Option<f64> {
        match *self { Float(f) => Some(f), _ => None }
    }

    /// Extracts the boolean value if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self { Boolean(b) => Some(b), _ => None }
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
        match *self { Datetime(ref s) => Some(s.as_slice()), _ => None }
    }

    /// Extracts the array value if it is an array.
    pub fn as_slice<'a>(&'a self) -> Option<&'a [Value]> {
        match *self { Array(ref s) => Some(s.as_slice()), _ => None }
    }

    /// Extracts the table value if it is a table.
    pub fn as_table<'a>(&'a self) -> Option<&'a Table> {
        match *self { Table(ref s) => Some(s), _ => None }
    }
}

impl FromStr for Value {
    fn from_str(s: &str) -> Option<Value> {
        Parser::new(s).parse().map(Table)
    }
}
