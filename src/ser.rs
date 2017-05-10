//! Serializing Rust structures into TOML.
//!
//! This module contains all the Serde support for serializing Rust structures
//! into TOML documents (as strings). Note that some top-level functions here
//! are also provided at the top of the crate.
//!
//! Note that the TOML format has a restriction that if a table itself contains
//! tables, all keys with non-table values must be emitted first. This is
//! typically easy to ensure happens when you're defining a `struct` as you can
//! reorder the fields manually, but when working with maps (such as `BTreeMap`
//! or `HashMap`) this can lead to serialization errors. In those situations you
//! may use the `tables_last` function in this module like so:
//!
//! ```rust
//! # #[macro_use] extern crate serde_derive;
//! # extern crate toml;
//! # use std::collections::HashMap;
//! #[derive(Serialize)]
//! struct Manifest {
//!     package: Package,
//!     #[serde(serialize_with = "toml::ser::tables_last")]
//!     dependencies: HashMap<String, Dependency>,
//! }
//! # type Package = String;
//! # type Dependency = String;
//! # fn main() {}
//! ```

use std::cell::Cell;
use std::error;
use std::fmt::{self, Write};
use std::marker;

use serde::ser;
use datetime::{SERDE_STRUCT_FIELD_NAME, SERDE_STRUCT_NAME};

/// Serialize the given data structure as a TOML byte vector.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, if `T` contains a map with non-string keys, or if `T` attempts to
/// serialize an unsupported datatype such as an enum, tuple, or tuple struct.
pub fn to_vec<T: ?Sized>(value: &T) -> Result<Vec<u8>, Error>
    where T: ser::Serialize,
{
    to_string(value).map(|e| e.into_bytes())
}

/// Serialize the given data structure as a String of TOML.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, if `T` contains a map with non-string keys, or if `T` attempts to
/// serialize an unsupported datatype such as an enum, tuple, or tuple struct.
///
/// # Examples
///
/// ```
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate toml;
///
/// #[derive(Serialize)]
/// struct Config {
///     database: Database,
/// }
///
/// #[derive(Serialize)]
/// struct Database {
///     ip: String,
///     port: Vec<u16>,
///     connection_max: u32,
///     enabled: bool,
/// }
///
/// fn main() {
///     let config = Config {
///         database: Database {
///             ip: "192.168.1.1".to_string(),
///             port: vec![8001, 8002, 8003],
///             connection_max: 5000,
///             enabled: false,
///         },
///     };
/// 
///     let toml = toml::to_string(&config).unwrap();
///     println!("{}", toml)
/// }
/// ```
pub fn to_string<T: ?Sized>(value: &T) -> Result<String, Error>
    where T: ser::Serialize,
{
    let mut dst = String::with_capacity(128);
    value.serialize(&mut Serializer::new(&mut dst))?;
    Ok(dst)
}

/// Errors that can occur when serializing a type.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    /// Indicates that a Rust type was requested to be serialized but it was not
    /// supported.
    ///
    /// Currently the TOML format does not support serializing types such as
    /// enums, tuples and tuple structs.
    UnsupportedType,

    /// The key of all TOML maps must be strings, but serialization was
    /// attempted where the key of a map was not a string.
    KeyNotString,

    /// Keys in maps are not allowed to have newlines.
    KeyNewline,

    /// Arrays in TOML must have a homogenous type, but a heterogeneous array
    /// was emitted.
    ArrayMixedType,

    /// All values in a TOML table must be emitted before further tables are
    /// emitted. If a value is emitted *after* a table then this error is
    /// generated.
    ValueAfterTable,

    /// A serialized date was invalid.
    DateInvalid,

    /// None was attempted to be serialized, but it's not supported.
    UnsupportedNone,

    /// A custom error which could be generated when serializing a particular
    /// type.
    Custom(String),

    #[doc(hidden)]
    __Nonexhaustive,
}

/// Serialization implementation for TOML.
///
/// This structure implements serialization support for TOML to serialize an
/// arbitrary type to TOML. Note that the TOML format does not support all
/// datatypes in Rust, such as enums, tuples, and tuple structs. These types
/// will generate an error when serialized.
///
/// Currently a serializer always writes its output to an in-memory `String`,
/// which is passed in when creating the serializer itself.
pub struct Serializer<'a> {
    dst: &'a mut String,
    state: State<'a>,
}

#[derive(Debug, Clone)]
enum State<'a> {
    Table {
        key: &'a str,
        parent: &'a State<'a>,
        first: &'a Cell<bool>,
        table_emitted: &'a Cell<bool>,
    },
    Array {
        parent: &'a State<'a>,
        first: &'a Cell<bool>,
        type_: &'a Cell<Option<&'static str>>,
    },
    End,
}

#[doc(hidden)]
pub struct SerializeSeq<'a: 'b, 'b> {
    ser: &'b mut Serializer<'a>,
    first: Cell<bool>,
    type_: Cell<Option<&'static str>>,
}

#[doc(hidden)]
pub enum SerializeTable<'a: 'b, 'b> {
    Datetime(&'b mut Serializer<'a>),
    Table {
        ser: &'b mut Serializer<'a>,
        key: String,
        first: Cell<bool>,
        table_emitted: Cell<bool>,
    }
}

impl<'a> Serializer<'a> {
    /// Creates a new serializer which will emit TOML into the buffer provided.
    ///
    /// The serializer can then be used to serialize a type after which the data
    /// will be present in `dst`.
    pub fn new(dst: &'a mut String) -> Serializer<'a> {
        Serializer {
            dst: dst,
            state: State::End,
        }
    }

    fn display<T: fmt::Display>(&mut self,
                                t: T,
                                type_: &'static str) -> Result<(), Error> {
        self.emit_key(type_)?;
        drop(write!(self.dst, "{}", t));
        if let State::Table { .. } = self.state {
            self.dst.push_str("\n");
        }
        Ok(())
    }

    fn emit_key(&mut self, type_: &'static str) -> Result<(), Error> {
        self.array_type(type_)?;
        let state = self.state.clone();
        self._emit_key(&state)
    }

    // recursive implementation of `emit_key` above
    fn _emit_key(&mut self, state: &State) -> Result<(), Error> {
        match *state {
            State::End => Ok(()),
            State::Array { parent, first, type_ } => {
                assert!(type_.get().is_some());
                if first.get() {
                    self._emit_key(parent)?;
                }
                self.emit_array(first)
            }
            State::Table { parent, first, table_emitted, key } => {
                if table_emitted.get() {
                    return Err(Error::ValueAfterTable)
                }
                if first.get() {
                    self.emit_table_header(parent)?;
                    first.set(false);
                }
                self.escape_key(key)?;
                self.dst.push_str(" = ");
                Ok(())
            }
        }
    }

    fn emit_array(&mut self, first: &Cell<bool>) -> Result<(), Error> {
        if first.get() {
            self.dst.push_str("[");
        } else {
            self.dst.push_str(", ");
        }
        Ok(())
    }

    fn array_type(&mut self, type_: &'static str) -> Result<(), Error> {
        let prev = match self.state {
            State::Array { type_, .. } => type_,
            _ => return Ok(()),
        };
        if let Some(prev) = prev.get() {
            if prev != type_ {
                return Err(Error::ArrayMixedType)
            }
        } else {
            prev.set(Some(type_));
        }
        Ok(())
    }

    fn escape_key(&mut self, key: &str) -> Result<(), Error> {
        let ok = key.chars().all(|c| {
            match c {
                'a' ... 'z' |
                'A' ... 'Z' |
                '0' ... '9' |
                '-' | '_' => true,
                _ => false,
            }
        });
        if ok {
            drop(write!(self.dst, "{}", key));
        } else {
            self.emit_str(key)?;
        }
        Ok(())
    }

    fn emit_str(&mut self, value: &str) -> Result<(), Error> {
        drop(write!(self.dst, "\""));
        for ch in value.chars() {
            match ch {
                '\u{8}' => drop(write!(self.dst, "\\b")),
                '\u{9}' => drop(write!(self.dst, "\\t")),
                '\u{a}' => drop(write!(self.dst, "\\n")),
                '\u{c}' => drop(write!(self.dst, "\\f")),
                '\u{d}' => drop(write!(self.dst, "\\r")),
                '\u{22}' => drop(write!(self.dst, "\\\"")),
                '\u{5c}' => drop(write!(self.dst, "\\\\")),
                c if c < '\u{1f}' => {
                    drop(write!(self.dst, "\\u{:04}", ch as u32))
                }
                ch => drop(write!(self.dst, "{}", ch)),
            }
        }
        drop(write!(self.dst, "\""));
        Ok(())
    }

    fn emit_table_header(&mut self, state: &State) -> Result<(), Error> {
        let array_of_tables = match *state {
            State::End => return Ok(()),
            State::Array { .. } => true,
            _ => false,
        };
        match *state {
            State::Table { first , .. } |
            State::Array { parent: &State::Table { first, .. }, .. } => {
                if !first.get() {
                    self.dst.push_str("\n");
                }
            }
            _ => {}
        }
        self.dst.push_str("[");
        if array_of_tables {
            self.dst.push_str("[");
        }
        self.emit_key_part(state)?;
        if array_of_tables {
            self.dst.push_str("]");
        }
        self.dst.push_str("]\n");
        Ok(())
    }

    fn emit_key_part(&mut self, key: &State) -> Result<bool, Error> {
        match *key {
            State::Array { parent, .. } => self.emit_key_part(parent),
            State::End => Ok(true),
            State::Table { key, parent, table_emitted, .. } => {
                table_emitted.set(true);
                let first  = self.emit_key_part(parent)?;
                if !first {
                    self.dst.push_str(".");
                }
                self.escape_key(key)?;
                Ok(false)
            }
        }
    }
}

impl<'a, 'b> ser::Serializer for &'b mut Serializer<'a> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeSeq<'a, 'b>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = SerializeTable<'a, 'b>;
    type SerializeStruct = SerializeTable<'a, 'b>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<(), Self::Error> {
        self.display(v, "bool")
    }

    fn serialize_i8(self, v: i8) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_i16(self, v: i16) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_i32(self, v: i32) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_i64(self, v: i64) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_u8(self, v: u8) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_u16(self, v: u16) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_u32(self, v: u32) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_u64(self, v: u64) -> Result<(), Self::Error> {
        self.display(v, "integer")
    }

    fn serialize_f32(mut self, v: f32) -> Result<(), Self::Error> {
        self.emit_key("float")?;
        drop(write!(self.dst, "{}", v));
        if v % 1.0 == 0.0 {
            drop(write!(self.dst, ".0"));
        }
        if let State::Table { .. } = self.state {
            self.dst.push_str("\n");
        }
        Ok(())
    }

    fn serialize_f64(mut self, v: f64) -> Result<(), Self::Error> {
        self.emit_key("float")?;
        drop(write!(self.dst, "{}", v));
        if v % 1.0 == 0.0 {
            drop(write!(self.dst, ".0"));
        }
        if let State::Table { .. } = self.state {
            self.dst.push_str("\n");
        }
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(mut self, value: &str) -> Result<(), Self::Error> {
        self.emit_key("string")?;
        self.emit_str(value)?;
        if let State::Table { .. } = self.state {
            self.dst.push_str("\n");
        }
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<(), Self::Error> {
        use serde::ser::Serialize;
        value.serialize(self)
    }

    fn serialize_none(self) -> Result<(), Self::Error> {
        Err(Error::UnsupportedNone)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<(), Self::Error>
        where T: ser::Serialize
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_unit_struct(self,
                             _name: &'static str)
                             -> Result<(), Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_unit_variant(self,
                              _name: &'static str,
                              _variant_index: u32,
                              variant: &'static str)
                              -> Result<(), Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T)
                                           -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(self,
                                            _name: &'static str,
                                            _variant_index: u32,
                                            _variant: &'static str,
                                            _value: &T)
                                            -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        Err(Error::UnsupportedType)
    }

    fn serialize_seq(mut self, _len: Option<usize>)
                     -> Result<Self::SerializeSeq, Self::Error> {
        self.array_type("array")?;
        Ok(SerializeSeq {
            ser: self,
            first: Cell::new(true),
            type_: Cell::new(None),
        })
    }

    fn serialize_tuple(self, _len: usize)
                       -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize)
                              -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_tuple_variant(self,
                               _name: &'static str,
                               _variant_index: u32,
                               _variant: &'static str,
                               _len: usize)
                               -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::UnsupportedType)
    }

    fn serialize_map(mut self, _len: Option<usize>)
                     -> Result<Self::SerializeMap, Self::Error> {
        self.array_type("table")?;
        Ok(SerializeTable::Table {
            ser: self,
            key: String::new(),
            first: Cell::new(true),
            table_emitted: Cell::new(false),
        })
    }

    fn serialize_struct(mut self, name: &'static str, _len: usize)
                        -> Result<Self::SerializeStruct, Self::Error> {
        if name == SERDE_STRUCT_NAME {
            self.array_type("datetime")?;
            Ok(SerializeTable::Datetime(self))
        } else {
            self.array_type("table")?;
            Ok(SerializeTable::Table {
                ser: self,
                key: String::new(),
                first: Cell::new(true),
                table_emitted: Cell::new(false),
            })
        }
    }

    fn serialize_struct_variant(self,
                                _name: &'static str,
                                _variant_index: u32,
                                _variant: &'static str,
                                _len: usize)
                                -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::UnsupportedType)
    }
}

impl<'a, 'b> ser::SerializeSeq for SerializeSeq<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where T: ser::Serialize,
    {
        value.serialize(&mut Serializer {
            dst: &mut *self.ser.dst,
            state: State::Array {
                parent: &self.ser.state,
                first: &self.first,
                type_: &self.type_,
            },
        })?;
        self.first.set(false);
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        match self.type_.get() {
            Some("table") => return Ok(()),
            Some(_) => self.ser.dst.push_str("]"),
            None => {
                assert!(self.first.get());
                self.ser.emit_key("array")?;
                self.ser.dst.push_str("[]")
            }
        }
        if let State::Table { .. } = self.ser.state {
            self.ser.dst.push_str("\n");
        }
        Ok(())
    }
}

impl<'a, 'b> ser::SerializeMap for SerializeTable<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, input: &T) -> Result<(), Error>
        where T: ser::Serialize,
    {
        match *self {
            SerializeTable::Datetime(_) => panic!(), // shouldn't be possible
            SerializeTable::Table { ref mut key, .. } => {
                key.truncate(0);
                *key = input.serialize(StringExtractor)?;
                if key.contains('\n') {
                    return Err(Error::KeyNewline)
                }
            }
        }
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
        where T: ser::Serialize,
    {
        match *self {
            SerializeTable::Datetime(_) => panic!(), // shouldn't be possible
            SerializeTable::Table {
                ref mut ser,
                ref key,
                ref first,
                ref table_emitted,
                ..
            } => {
                let res = value.serialize(&mut Serializer {
                    dst: &mut *ser.dst,
                    state: State::Table {
                        key: key,
                        parent: &ser.state,
                        first: first,
                        table_emitted: table_emitted,
                    },
                });
                match res {
                    Ok(()) => first.set(false),
                    Err(Error::UnsupportedNone) => {},
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        match self {
            SerializeTable::Datetime(_) => panic!(), // shouldn't be possible
            SerializeTable::Table { mut ser, first, ..  } => {
                if first.get() {
                    let state = ser.state.clone();
                    ser.emit_table_header(&state)?;
                }
            }
        }
        Ok(())
    }
}

impl<'a, 'b> ser::SerializeStruct for SerializeTable<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T)
                                  -> Result<(), Error>
        where T: ser::Serialize,
    {
        match *self {
            SerializeTable::Datetime(ref mut ser) => {
                if key == SERDE_STRUCT_FIELD_NAME {
                    value.serialize(DateStrEmitter(&mut *ser))?;
                } else {
                    return Err(Error::DateInvalid)
                }
            }
            SerializeTable::Table {
                ref mut ser,
                ref first,
                ref table_emitted,
                ..
            } => {
                let res = value.serialize(&mut Serializer {
                    dst: &mut *ser.dst,
                    state: State::Table {
                        key: key,
                        parent: &ser.state,
                        first: first,
                        table_emitted: table_emitted,
                    },
                });
                match res {
                    Ok(()) => first.set(false),
                    Err(Error::UnsupportedNone) => {},
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }

    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

struct DateStrEmitter<'a: 'b, 'b>(&'b mut Serializer<'a>);

impl<'a, 'b> ser::Serializer for DateStrEmitter<'a, 'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, _v: bool) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_i8(self, _v: i8) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_i16(self, _v: i16) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_i32(self, _v: i32) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_i64(self, _v: i64) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_u8(self, _v: u8) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_u16(self, _v: u16) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_u32(self, _v: u32) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_u64(self, _v: u64) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_f32(self, _v: f32) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_f64(self, _v: f64) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_char(self, _v: char) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_str(self, value: &str) -> Result<(), Self::Error> {
        self.0.display(value, "datetime")?;
        Ok(())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_none(self) -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<(), Self::Error>
        where T: ser::Serialize
    {
        Err(Error::KeyNotString)
    }

    fn serialize_unit(self) -> Result<(), Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_unit_struct(self,
                             _name: &'static str)
                             -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_unit_variant(self,
                              _name: &'static str,
                              _variant_index: u32,
                              _variant: &'static str)
                              -> Result<(), Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, _value: &T)
                                           -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        Err(Error::DateInvalid)
    }

    fn serialize_newtype_variant<T: ?Sized>(self,
                                            _name: &'static str,
                                            _variant_index: u32,
                                            _variant: &'static str,
                                            _value: &T)
                                            -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        Err(Error::DateInvalid)
    }

    fn serialize_seq(self, _len: Option<usize>)
                     -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_tuple(self, _len: usize)
                       -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize)
                              -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_tuple_variant(self,
                               _name: &'static str,
                               _variant_index: u32,
                               _variant: &'static str,
                               _len: usize)
                               -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_map(self, _len: Option<usize>)
                     -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize)
                        -> Result<Self::SerializeStruct, Self::Error> {
        Err(Error::DateInvalid)
    }

    fn serialize_struct_variant(self,
                                _name: &'static str,
                                _variant_index: u32,
                                _variant: &'static str,
                                _len: usize)
                                -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::DateInvalid)
    }
}

struct StringExtractor;

impl ser::Serializer for StringExtractor {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;

    fn serialize_bool(self, _v: bool) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_i8(self, _v: i8) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_i16(self, _v: i16) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_i32(self, _v: i32) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_i64(self, _v: i64) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_u8(self, _v: u8) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_u16(self, _v: u16) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_u32(self, _v: u32) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_u64(self, _v: u64) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_f32(self, _v: f32) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_f64(self, _v: f64) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_char(self, _v: char) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_str(self, value: &str) -> Result<String, Self::Error> {
        Ok(value.to_string())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_none(self) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<String, Self::Error>
        where T: ser::Serialize
    {
        Err(Error::KeyNotString)
    }

    fn serialize_unit(self) -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_unit_struct(self,
                             _name: &'static str)
                             -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_unit_variant(self,
                              _name: &'static str,
                              _variant_index: u32,
                              _variant: &'static str)
                              -> Result<String, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, _value: &T)
                                           -> Result<String, Self::Error>
        where T: ser::Serialize,
    {
        Err(Error::KeyNotString)
    }

    fn serialize_newtype_variant<T: ?Sized>(self,
                                            _name: &'static str,
                                            _variant_index: u32,
                                            _variant: &'static str,
                                            _value: &T)
                                            -> Result<String, Self::Error>
        where T: ser::Serialize,
    {
        Err(Error::KeyNotString)
    }

    fn serialize_seq(self, _len: Option<usize>)
                     -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_tuple(self, _len: usize)
                       -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize)
                              -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_tuple_variant(self,
                               _name: &'static str,
                               _variant_index: u32,
                               _variant: &'static str,
                               _len: usize)
                               -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_map(self, _len: Option<usize>)
                     -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize)
                        -> Result<Self::SerializeStruct, Self::Error> {
        Err(Error::KeyNotString)
    }

    fn serialize_struct_variant(self,
                                _name: &'static str,
                                _variant_index: u32,
                                _variant: &'static str,
                                _len: usize)
                                -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::KeyNotString)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::UnsupportedType => "unsupported Rust type".fmt(f),
            Error::KeyNotString => "map key was not a string".fmt(f),
            Error::KeyNewline => "map keys cannot contain newlines".fmt(f),
            Error::ArrayMixedType => "arrays cannot have mixed types".fmt(f),
            Error::ValueAfterTable => "values must be emitted before tables".fmt(f),
            Error::DateInvalid => "a serialize date was invalid".fmt(f),
            Error::UnsupportedNone => "unsupported None value".fmt(f),
            Error::Custom(ref s) => s.fmt(f),
            Error::__Nonexhaustive => panic!(),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::UnsupportedType => "unsupported Rust type",
            Error::KeyNotString => "map key was not a string",
            Error::KeyNewline => "map keys cannot contain newlines",
            Error::ArrayMixedType => "arrays cannot have mixed types",
            Error::ValueAfterTable => "values must be emitted before tables",
            Error::DateInvalid => "a serialized date was invalid",
            Error::UnsupportedNone => "unsupported None value",
            Error::Custom(_) => "custom error",
            Error::__Nonexhaustive => panic!(),
        }
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Error {
        Error::Custom(msg.to_string())
    }
}

enum Category {
    Primitive,
    Array,
    Table,
}

/// Convenience function to serialize items in a map in an order valid with
/// TOML.
///
/// TOML carries the restriction that keys in a table must be serialized last if
/// their value is a table itself. This isn't always easy to guarantee, so this
/// helper can be used like so:
///
/// ```rust
/// # #[macro_use] extern crate serde_derive;
/// # extern crate toml;
/// # use std::collections::HashMap;
/// #[derive(Serialize)]
/// struct Manifest {
///     package: Package,
///     #[serde(serialize_with = "toml::ser::tables_last")]
///     dependencies: HashMap<String, Dependency>,
/// }
/// # type Package = String;
/// # type Dependency = String;
/// # fn main() {}
/// ```
pub fn tables_last<'a, I, K, V, S>(data: &'a I, serializer: S)
                                   -> Result<S::Ok, S::Error>
    where &'a I: IntoIterator<Item = (K, V)>,
          K: ser::Serialize,
          V: ser::Serialize,
          S: ser::Serializer
{
    use serde::ser::SerializeMap;

    let mut map = serializer.serialize_map(None)?;
    for (k, v) in data {
        if let Category::Primitive = v.serialize(Categorize::new())? {
            map.serialize_entry(&k, &v)?;
        }
    }
    for (k, v) in data {
        if let Category::Array = v.serialize(Categorize::new())? {
            map.serialize_entry(&k, &v)?;
        }
    }
    for (k, v) in data {
        if let Category::Table = v.serialize(Categorize::new())? {
            map.serialize_entry(&k, &v)?;
        }
    }
    map.end()
}

struct Categorize<E>(marker::PhantomData<E>);

impl<E> Categorize<E> {
    fn new() -> Self {
        Categorize(marker::PhantomData)
    }
}

impl<E: ser::Error> ser::Serializer for Categorize<E> {
    type Ok = Category;
    type Error = E;
    type SerializeSeq = Self;
    type SerializeTuple = ser::Impossible<Category, E>;
    type SerializeTupleStruct = ser::Impossible<Category, E>;
    type SerializeTupleVariant = ser::Impossible<Category, E>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = ser::Impossible<Category, E>;

    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Primitive)
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Array)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_some<T: ?Sized + ser::Serialize>(self, v: &T) -> Result<Self::Ok, Self::Error> {
        v.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(self, _: &'static str, v: &T) -> Result<Self::Ok, Self::Error> {
        v.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(self, _: &'static str, _: u32, _: &'static str, _: &T) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_tuple_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Self::Error> {
        Ok(self)
    }

    fn serialize_struct_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom("unsupported"))
    }
}

impl<E: ser::Error> ser::SerializeSeq for Categorize<E> {
    type Ok = Category;
    type Error = E;

    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, _: &T)
                                                     -> Result<(), Self::Error> {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Array)
    }
}

impl<E: ser::Error> ser::SerializeMap for Categorize<E> {
    type Ok = Category;
    type Error = E;

    fn serialize_key<T: ?Sized + ser::Serialize>(&mut self, _: &T)
                                                 -> Result<(), Self::Error> {
        Ok(())
    }

    fn serialize_value<T: ?Sized + ser::Serialize>(&mut self, _: &T)
                                                   -> Result<(), Self::Error> {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Table)
    }
}

impl<E: ser::Error> ser::SerializeStruct for Categorize<E> {
    type Ok = Category;
    type Error = E;

    fn serialize_field<T: ?Sized>(&mut self,
                                  _: &'static str,
                                  _: &T) -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Category::Table)
    }
}
