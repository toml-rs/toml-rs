#![allow(warnings)]

use std::collections::HashMap;
use std::mem;

use serialize;
use {Value, Table, Array, String, Integer, Float, Boolean};

/// A structure to transform Rust values into TOML values.
///
/// This encoder implements the serialization `Encoder` interface, allowing
/// `Encodable` rust types to be fed into the encoder. The output of this
/// encoder is a TOML `Table` structure. The resulting TOML can be stringified
/// if necessary.
///
/// # Example
///
/// ```
/// extern crate serialize;
/// extern crate toml;
///
/// # fn main() {
/// use toml::{Encoder, Integer};
/// use serialize::Encodable;
///
/// #[deriving(Encodable)]
/// struct MyStruct { foo: int, bar: String }
/// let my_struct = MyStruct { foo: 4, bar: "hello!".to_string() };
///
/// let mut e = Encoder::new();
/// my_struct.encode(&mut e).unwrap();
///
/// assert_eq!(e.toml.find_equiv(&"foo"), Some(&Integer(4)))
/// # }
/// ```
pub struct Encoder {
    /// Output TOML that is emitted. The current version of this encoder forces
    /// the top-level representation of a structure to be a table.
    ///
    /// This field can be used to extract the return value after feeding a value
    /// into this `Encoder`.
    pub toml: Table,
    state: EncoderState,
}

pub struct Decoder {
    toml: Option<Value>,
}

/// Enumeration of errors which can occur while encoding a rust value into a
/// TOML value.
#[deriving(Show)]
pub enum Error {
    /// Indication that a key was needed when a value was emitted, but no key
    /// was previously emitted.
    NeedsKey,
    /// Indication that a key was emitted, but not value was emitted.
    NoValue,
    /// Indicates that a map key was attempted to be emitted at an invalid
    /// location.
    InvalidMapKeyLocation,
    /// Indicates that a type other than a string was attempted to be used as a
    /// map key type.
    InvalidMapKeyType,
    /// Indicates that a type was decoded against a TOML value of a different
    /// type.
    InvalidType,
    /// Indicates that a field was attempted to be read that does not exist.
    MissingField,
}

#[deriving(PartialEq, Show)]
enum EncoderState {
    Start,
    NextKey(String),
    NextArray(Vec<Value>),
    NextMapKey,
}

/// Encodes an encodable value into a TOML value.
///
/// This function expects the type given to represent a TOML table in some form.
/// If encoding encounters an error, then this function will fail the task.
pub fn encode<T: serialize::Encodable<Encoder, Error>>(t: &T) -> Value {
    let mut e = Encoder::new();
    t.encode(&mut e).unwrap();
    Table(e.toml)
}

/// Encodes an encodable value into a TOML string.
///
/// This function expects the type given to represent a TOML table in some form.
/// If encoding encounters an error, then this function will fail the task.
pub fn encode_str<T: serialize::Encodable<Encoder, Error>>(t: &T) -> String {
    format!("{}", encode(t))
}

impl Encoder {
    /// Constructs a new encoder which will emit to the given output stream.
    pub fn new() -> Encoder {
        Encoder { state: Start, toml: HashMap::new() }
    }

    fn emit_value(&mut self, v: Value) -> Result<(), Error> {
        match mem::replace(&mut self.state, Start) {
            NextKey(key) => { self.toml.insert(key, v); Ok(()) }
            NextArray(mut vec) => {
                // TODO: validate types
                vec.push(v);
                self.state = NextArray(vec);
                Ok(())
            }
            NextMapKey => {
                match v {
                    String(s) => { self.state = NextKey(s); Ok(()) }
                    _ => Err(InvalidMapKeyType)
                }
            }
            _ => Err(NeedsKey)
        }
    }
}

impl serialize::Encoder<Error> for Encoder {
    fn emit_nil(&mut self) -> Result<(), Error> { Ok(()) }
    fn emit_uint(&mut self, v: uint) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_u8(&mut self, v: u8) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_u16(&mut self, v: u16) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_u32(&mut self, v: u32) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_u64(&mut self, v: u64) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_int(&mut self, v: int) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_i8(&mut self, v: i8) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_i16(&mut self, v: i16) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_i32(&mut self, v: i32) -> Result<(), Error> {
        self.emit_i64(v as i64)
    }
    fn emit_i64(&mut self, v: i64) -> Result<(), Error> {
        self.emit_value(Integer(v))
    }
    fn emit_bool(&mut self, v: bool) -> Result<(), Error> {
        self.emit_value(Boolean(v))
    }
    fn emit_f32(&mut self, v: f32) -> Result<(), Error> { self.emit_f64(v as f64) }
    fn emit_f64(&mut self, v: f64) -> Result<(), Error> {
        self.emit_value(Float(v))
    }
    fn emit_char(&mut self, v: char) -> Result<(), Error> {
        self.emit_str(v.to_str().as_slice())
    }
    fn emit_str(&mut self, v: &str) -> Result<(), Error> {
        self.emit_value(String(v.to_str()))
    }
    fn emit_enum(&mut self, _name: &str,
                 _f: |&mut Encoder| -> Result<(), Error>) -> Result<(), Error> {
        fail!()
    }
    fn emit_enum_variant(&mut self, _v_name: &str, _v_id: uint, _len: uint,
                         _f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        fail!()
    }
    fn emit_enum_variant_arg(&mut self, _a_idx: uint,
                             _f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        fail!()
    }
    fn emit_enum_struct_variant(&mut self, _v_name: &str, _v_id: uint,
                                _len: uint,
                                _f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        fail!()
    }
    fn emit_enum_struct_variant_field(&mut self, _f_name: &str, _f_idx: uint,
                                      _f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        fail!()
    }
    fn emit_struct(&mut self, _name: &str, _len: uint,
                   f: |&mut Encoder| -> Result<(), Error>) -> Result<(), Error> {
        match mem::replace(&mut self.state, Start) {
            NextKey(key) => {
                let mut nested = Encoder::new();
                try!(f(&mut nested));
                self.toml.insert(key, Table(nested.toml));
                Ok(())
            }
            NextArray(mut arr) => {
                let mut nested = Encoder::new();
                try!(f(&mut nested));
                arr.push(Table(nested.toml));
                self.state = NextArray(arr);
                Ok(())
            }
            Start => f(self),
            NextMapKey => Err(InvalidMapKeyLocation),
        }
    }
    fn emit_struct_field(&mut self, f_name: &str, _f_idx: uint,
                         f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        let old = mem::replace(&mut self.state, NextKey(f_name.to_str()));
        try!(f(self));
        if self.state != Start {
            println!("{}", self.state);
            return Err(NoValue)
        }
        self.state = old;
        Ok(())
    }
    fn emit_tuple(&mut self, len: uint,
                  f: |&mut Encoder| -> Result<(), Error>) -> Result<(), Error> {
        self.emit_seq(len, f)
    }
    fn emit_tuple_arg(&mut self, idx: uint,
                      f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        self.emit_seq_elt(idx, f)
    }
    fn emit_tuple_struct(&mut self, _name: &str, _len: uint,
                         _f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        unimplemented!()
    }
    fn emit_tuple_struct_arg(&mut self, _f_idx: uint,
                             _f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        unimplemented!()
    }
    fn emit_option(&mut self,
                   f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        f(self)
    }
    fn emit_option_none(&mut self) -> Result<(), Error> {
        match mem::replace(&mut self.state, Start) {
            Start => unreachable!(),
            NextKey(_) => Ok(()),
            NextArray(..) => fail!("how to encode None in an array?"),
            NextMapKey => Err(InvalidMapKeyLocation),
        }
    }
    fn emit_option_some(&mut self,
                        f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        f(self)
    }
    fn emit_seq(&mut self, _len: uint,
                f: |this: &mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        let old = mem::replace(&mut self.state, NextArray(Vec::new()));
        try!(f(self));
        match mem::replace(&mut self.state, old) {
            NextArray(v) => self.emit_value(Array(v)),
            _ => unreachable!(),
        }
    }
    fn emit_seq_elt(&mut self, _idx: uint,
                    f: |this: &mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        f(self)
    }
    fn emit_map(&mut self, len: uint,
                f: |&mut Encoder| -> Result<(), Error>) -> Result<(), Error> {
        self.emit_struct("foo", len, f)
    }
    fn emit_map_elt_key(&mut self, _idx: uint,
                        f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        match mem::replace(&mut self.state, NextMapKey) {
            Start => {}
            _ => return Err(InvalidMapKeyLocation),
        }
        try!(f(self));
        match self.state {
            NextKey(_) => Ok(()),
            _ => Err(InvalidMapKeyLocation),
        }
    }
    fn emit_map_elt_val(&mut self, _idx: uint,
                        f: |&mut Encoder| -> Result<(), Error>)
        -> Result<(), Error>
    {
        f(self)
    }
}

impl Decoder {
    pub fn new(toml: Value) -> Decoder {
        Decoder { toml: Some(toml) }
    }
}

impl serialize::Decoder<Error> for Decoder {
    fn read_nil(&mut self) -> Result<(), Error> {
        match self.toml {
            Some(String(ref s)) if s.len() == 0 => Ok(()),
            _ => Err(InvalidType),
        }
    }
    fn read_uint(&mut self) -> Result<uint, Error> {
        self.read_i64().map(|i| i as uint)
    }
    fn read_u64(&mut self) -> Result<u64, Error> {
        self.read_i64().map(|i| i as u64)
    }
    fn read_u32(&mut self) -> Result<u32, Error> {
        self.read_i64().map(|i| i as u32)
    }
    fn read_u16(&mut self) -> Result<u16, Error> {
        self.read_i64().map(|i| i as u16)
    }
    fn read_u8(&mut self) -> Result<u8, Error> {
        self.read_i64().map(|i| i as u8)
    }
    fn read_int(&mut self) -> Result<int, Error> {
        self.read_i64().map(|i| i as int)
    }
    fn read_i64(&mut self) -> Result<i64, Error> {
        match self.toml {
            Some(Integer(i)) => Ok(i),
            _ => Err(InvalidType),
        }
    }
    fn read_i32(&mut self) -> Result<i32, Error> {
        self.read_i64().map(|i| i as i32)
    }
    fn read_i16(&mut self) -> Result<i16, Error> {
        self.read_i64().map(|i| i as i16)
    }
    fn read_i8(&mut self) -> Result<i8, Error> {
        self.read_i64().map(|i| i as i8)
    }
    fn read_bool(&mut self) -> Result<bool, Error> {
        match self.toml {
            Some(Boolean(b)) => Ok(b),
            _ => Err(InvalidType),
        }
    }
    fn read_f64(&mut self) -> Result<f64, Error> {
        match self.toml {
            Some(Float(f)) => Ok(f),
            _ => Err(InvalidType),
        }
    }
    fn read_f32(&mut self) -> Result<f32, Error> {
        self.read_f64().map(|f| f as f32)
    }
    fn read_char(&mut self) -> Result<char, Error> {
        match self.toml {
            Some(String(ref s)) if s.as_slice().char_len() == 1 =>
                Ok(s.as_slice().char_at(0)),
            _ => Err(InvalidType),
        }
    }
    fn read_str(&mut self) -> Result<String, Error> {
        match self.toml.take() {
            Some(String(s)) => Ok(s),
            toml => { self.toml = toml; Err(InvalidType) }
        }
    }

    // Compound types:
    fn read_enum<T>(&mut self, name: &str,
                    f: |&mut Decoder| -> Result<T, Error>) -> Result<T, Error> {
        fail!()
    }

    fn read_enum_variant<T>(&mut self,
                            names: &[&str],
                            f: |&mut Decoder, uint| -> Result<T, Error>)
                            -> Result<T, Error> {
        fail!()
    }
    fn read_enum_variant_arg<T>(&mut self,
                                a_idx: uint,
                                f: |&mut Decoder| -> Result<T, Error>)
                                -> Result<T, Error> {
        fail!()
    }

    fn read_enum_struct_variant<T>(&mut self,
                                   names: &[&str],
                                   f: |&mut Decoder, uint| -> Result<T, Error>)
                                   -> Result<T, Error> {
        fail!()
    }
    fn read_enum_struct_variant_field<T>(&mut self,
                                         f_name: &str,
                                         f_idx: uint,
                                         f: |&mut Decoder| -> Result<T, Error>)
                                         -> Result<T, Error> {
        fail!()
    }

    fn read_struct<T>(&mut self, _s_name: &str, _len: uint,
                      f: |&mut Decoder| -> Result<T, Error>)
        -> Result<T, Error>
    {
        match self.toml {
            Some(Table(..)) => f(self),
            _ => Err(InvalidType),
        }
    }
    fn read_struct_field<T>(&mut self,
                            f_name: &str,
                            f_idx: uint,
                            f: |&mut Decoder| -> Result<T, Error>)
                            -> Result<T, Error> {
        match self.toml {
            Some(Table(ref mut table)) => {
                match table.pop(&f_name.to_string()) {
                    Some(field) => f(&mut Decoder::new(field)),
                    None => f(&mut Decoder { toml: None }),
                }
            }
            _ => Err(InvalidType)
        }
    }

    fn read_tuple<T>(&mut self,
                     f: |&mut Decoder, uint| -> Result<T, Error>)
        -> Result<T, Error>
    {
        self.read_seq(f)
    }
    fn read_tuple_arg<T>(&mut self, a_idx: uint,
                         f: |&mut Decoder| -> Result<T, Error>)
        -> Result<T, Error>
    {
        self.read_seq_elt(a_idx, f)
    }

    fn read_tuple_struct<T>(&mut self,
                            s_name: &str,
                            f: |&mut Decoder, uint| -> Result<T, Error>)
        -> Result<T, Error>
    {
        fail!()
    }
    fn read_tuple_struct_arg<T>(&mut self,
                                a_idx: uint,
                                f: |&mut Decoder| -> Result<T, Error>)
        -> Result<T, Error>
    {
        fail!()
    }

    // Specialized types:
    fn read_option<T>(&mut self,
                      f: |&mut Decoder, bool| -> Result<T, Error>)
        -> Result<T, Error>
    {
        match self.toml {
            Some(..) => f(self, true),
            None => f(self, false),
        }
    }

    fn read_seq<T>(&mut self, f: |&mut Decoder, uint| -> Result<T, Error>)
        -> Result<T, Error>
    {
        let len = match self.toml {
            Some(Array(ref arr)) => arr.len(),
            _ => return Err(InvalidType),
        };
        f(self, len)
    }
    fn read_seq_elt<T>(&mut self, idx: uint, f: |&mut Decoder| -> Result<T, Error>)
        -> Result<T, Error>
    {
        match self.toml {
            Some(Array(ref mut arr)) => {
                f(&mut Decoder::new(mem::replace(arr.get_mut(idx), Integer(0))))
            }
            _ => Err(InvalidType),
        }
    }

    fn read_map<T>(&mut self, f: |&mut Decoder, uint| -> Result<T, Error>)
        -> Result<T, Error>
    {
        let len = match self.toml {
            Some(Table(ref table)) => table.len(),
            _ => return Err(InvalidType),
        };
        f(self, len)
    }
    fn read_map_elt_key<T>(&mut self, idx: uint,
                           f: |&mut Decoder| -> Result<T, Error>)
        -> Result<T, Error>
    {
        match self.toml {
            Some(Table(ref table)) => {
                match table.keys().skip(idx).next() {
                    Some(key) => {
                        f(&mut Decoder::new(String(key.to_str())))
                    }
                    None => Err(InvalidType),
                }
            }
            _ => Err(InvalidType),
        }
    }
    fn read_map_elt_val<T>(&mut self, idx: uint,
                           f: |&mut Decoder| -> Result<T, Error>)
        -> Result<T, Error>
    {
        match self.toml {
            Some(Table(ref table)) => {
                match table.values().skip(idx).next() {
                    Some(key) => {
                        // XXX: this shouldn't clone
                        f(&mut Decoder::new(key.clone()))
                    }
                    None => Err(InvalidType),
                }
            }
            _ => Err(InvalidType),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use serialize::{Encodable, Decodable};

    use super::{Encoder, Decoder};
    use {Table, Integer, String, Array, Float};

    macro_rules! encode( ($t:expr) => ({
        let mut e = Encoder::new();
        $t.encode(&mut e).unwrap();
        e.toml
    }) )

    macro_rules! decode( ($t:expr) => ({
        let mut d = Decoder::new($t);
        Decodable::decode(&mut d).unwrap()
    }) )

    macro_rules! map( ($($k:ident: $v:expr),*) => ({
        let mut _m = HashMap::new();
        $(_m.insert(stringify!($k).to_str(), $v);)*
        _m
    }) )

    #[test]
    fn smoke() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo { a: int }

        let v = Foo { a: 2 };
        assert_eq!(encode!(v), map! { a: Integer(2) });
        assert_eq!(v, decode!(Table(encode!(v))));
    }

    #[test]
    fn nested() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo { a: int, b: Bar }
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Bar { a: String }

        let v = Foo { a: 2, b: Bar { a: "test".to_string() } };
        assert_eq!(encode!(v),
                   map! {
                       a: Integer(2),
                       b: Table(map! {
                           a: String("test".to_string())
                       })
                   });
        assert_eq!(v, decode!(Table(encode!(v))));
    }

    #[test]
    fn array() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo { a: Vec<int> }

        let v = Foo { a: vec![1, 2, 3, 4] };
        assert_eq!(encode!(v),
                   map! {
                       a: Array(vec![
                            Integer(1),
                            Integer(2),
                            Integer(3),
                            Integer(4)
                       ])
                   });
        assert_eq!(v, decode!(Table(encode!(v))));
    }

    #[test]
    fn tuple() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo { a: (int, int, int, int) }

        let v = Foo { a: (1, 2, 3, 4) };
        assert_eq!(encode!(v),
                   map! {
                       a: Array(vec![
                            Integer(1),
                            Integer(2),
                            Integer(3),
                            Integer(4)
                       ])
                   });
        assert_eq!(v, decode!(Table(encode!(v))));
    }

    #[test]
    fn inner_structs_with_options() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo {
            a: Option<Box<Foo>>,
            b: Bar,
        }
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Bar {
            a: String,
            b: f64,
        }

        let v = Foo {
            a: Some(box Foo {
                a: None,
                b: Bar { a: "foo".to_string(), b: 4.5 },
            }),
            b: Bar { a: "bar".to_string(), b: 1.0 },
        };
        assert_eq!(encode!(v),
                   map! {
                       a: Table(map! {
                           b: Table(map! {
                               a: String("foo".to_string()),
                               b: Float(4.5)
                           })
                       }),
                       b: Table(map! {
                           a: String("bar".to_string()),
                           b: Float(1.0)
                       })
                   });
        assert_eq!(v, decode!(Table(encode!(v))));
    }

    #[test]
    fn hashmap() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo {
            map: HashMap<String, int>,
            set: HashSet<char>,
        }

        let v = Foo {
            map: {
                let mut m = HashMap::new();
                m.insert("foo".to_string(), 10);
                m.insert("bar".to_string(), 4);
                m
            },
            set: {
                let mut s = HashSet::new();
                s.insert('a');
                s
            },
        };
        assert_eq!(encode!(v),
            map! {
                map: Table(map! {
                    foo: Integer(10),
                    bar: Integer(4)
                }),
                set: Array(vec![String("a".to_str())])
            }
        );
        assert_eq!(v, decode!(Table(encode!(v))));
    }

    #[test]
    fn tuple_struct() {
        #[deriving(Encodable, Decodable, PartialEq, Show)]
        struct Foo(int, String, f64);

        let v = Foo(1, "foo".to_string(), 4.5);
        assert_eq!(
            encode!(v),
            map! {
                _field0: Integer(1),
                _field1: String("foo".to_string()),
                _field2: Float(4.5)
            }
        );
        assert_eq!(v, decode!(Table(encode!(v))));
    }
}
