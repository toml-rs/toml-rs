//! Definition of a TOML spanned value

use std::fmt;
use std::mem::discriminant;
use std::ops;
use std::str::FromStr;

use serde::de;
use serde::ser;

use crate::datetime::{self, DatetimeFromString};
pub use crate::datetime::{Datetime, DatetimeParseError};
use crate::spanned::{self, Spanned};

pub use crate::map::Map;

/// Type representing a value with a span
pub type SpannedValue = Spanned<ValueKind>;

/// Representation of a TOML value.
#[derive(PartialEq, Clone, Debug)]
pub enum ValueKind {
    /// Represents a TOML string
    String(String),
    /// Represents a TOML integer
    Integer(i64),
    /// Represents a TOML float
    Float(f64),
    /// Represents a TOML boolean
    Boolean(bool),
    /// Represents a TOML datetime
    Datetime(Datetime),
    /// Represents a TOML array
    Array(Array),
    /// Represents a TOML table
    Table(Table),
}

/// Type representing a TOML array, payload of the `ValueKind::Array` variant
pub type Array = Vec<SpannedValue>;

/// Type representing a TOML table, payload of the `ValueKind::Table` variant.
/// By default it is backed by a BTreeMap, enable the `preserve_order` feature
/// to use a LinkedHashMap instead.
pub type Table = Map<Spanned<String>, SpannedValue>;

impl ValueKind {
    /* /// Interpret a `toml::ValueKind` as an instance of type `T`.
    ///
    /// This conversion can fail if the structure of the `ValueKind` does not match the
    /// structure expected by `T`, for example if `T` is a struct type but the
    /// `ValueKind` contains something other than a TOML table. It can also fail if the
    /// structure is correct but `T`'s implementation of `Deserialize` decides that
    /// something is wrong with the data, for example required struct fields are
    /// missing from the TOML map or some number is too big to fit in the expected
    /// primitive type.
    pub fn try_into<'de, T>(self) -> Result<T, crate::de::Error>
    where
        T: de::Deserialize<'de>,
    {
        de::Deserialize::deserialize(self)
    }*/

    /// Index into a TOML array or map. A string index can be used to access a
    /// value in a map, and a usize index can be used to access an element of an
    /// array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    pub fn get<I: Index>(&self, index: I) -> Option<&SpannedValue> {
        index.index(self)
    }

    /// Mutably index into a TOML array or map. A string index can be used to
    /// access a value in a map, and a usize index can be used to access an
    /// element of an array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the array.
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut SpannedValue> {
        index.index_mut(self)
    }

    /// Extracts the integer value if it is an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            ValueKind::Integer(i) => Some(i),
            _ => None,
        }
    }

    /// Tests whether this value is an integer.
    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }

    /// Extracts the float value if it is a float.
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            ValueKind::Float(f) => Some(f),
            _ => None,
        }
    }

    /// Tests whether this value is a float.
    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    /// Extracts the boolean value if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            ValueKind::Boolean(b) => Some(b),
            _ => None,
        }
    }

    /// Tests whether this value is a boolean.
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Extracts the string of this value if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            ValueKind::String(ref s) => Some(&**s),
            _ => None,
        }
    }

    /// Tests if this value is a string.
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Extracts the datetime value if it is a datetime.
    ///
    /// Note that a parsed TOML value will only contain ISO 8601 dates. An
    /// example date is:
    ///
    /// ```notrust
    /// 1979-05-27T07:32:00Z
    /// ```
    pub fn as_datetime(&self) -> Option<&Datetime> {
        match *self {
            ValueKind::Datetime(ref s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is a datetime.
    pub fn is_datetime(&self) -> bool {
        self.as_datetime().is_some()
    }

    /// Extracts the array value if it is an array.
    pub fn as_array(&self) -> Option<&Vec<SpannedValue>> {
        match *self {
            ValueKind::Array(ref s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the array value if it is an array.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<SpannedValue>> {
        match *self {
            ValueKind::Array(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is an array.
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Extracts the table value if it is a table.
    pub fn as_table(&self) -> Option<&Table> {
        match *self {
            ValueKind::Table(ref s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the table value if it is a table.
    pub fn as_table_mut(&mut self) -> Option<&mut Table> {
        match *self {
            ValueKind::Table(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// Tests whether this value is a table.
    pub fn is_table(&self) -> bool {
        self.as_table().is_some()
    }

    /// Tests whether this and another value have the same type.
    pub fn same_type(&self, other: &ValueKind) -> bool {
        discriminant(self) == discriminant(other)
    }

    /// Returns a human-readable representation of the type of this value.
    pub fn type_str(&self) -> &'static str {
        match *self {
            ValueKind::String(..) => "string",
            ValueKind::Integer(..) => "integer",
            ValueKind::Float(..) => "float",
            ValueKind::Boolean(..) => "boolean",
            ValueKind::Datetime(..) => "datetime",
            ValueKind::Array(..) => "array",
            ValueKind::Table(..) => "table",
        }
    }
}

impl<I> ops::Index<I> for ValueKind
where
    I: Index,
{
    type Output = SpannedValue;

    fn index(&self, index: I) -> &SpannedValue {
        self.get(index).expect("index not found")
    }
}

impl<I> ops::IndexMut<I> for ValueKind
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut SpannedValue {
        self.get_mut(index).expect("index not found")
    }
}

impl<'a> From<&'a str> for ValueKind {
    #[inline]
    fn from(val: &'a str) -> ValueKind {
        ValueKind::String(val.to_string())
    }
}

impl<V: Into<SpannedValue>> From<Vec<V>> for ValueKind {
    fn from(val: Vec<V>) -> ValueKind {
        ValueKind::Array(val.into_iter().map(|v| v.into()).collect())
    }
}

macro_rules! impl_into_value {
    ($variant:ident : $T:ty) => {
        impl From<$T> for ValueKind {
            #[inline]
            fn from(val: $T) -> ValueKind {
                ValueKind::$variant(val.into())
            }
        }
    };
}

impl_into_value!(String: String);
impl_into_value!(Integer: i64);
impl_into_value!(Integer: i32);
impl_into_value!(Integer: i8);
impl_into_value!(Integer: u8);
impl_into_value!(Integer: u32);
impl_into_value!(Float: f64);
impl_into_value!(Float: f32);
impl_into_value!(Boolean: bool);
impl_into_value!(Datetime: Datetime);
impl_into_value!(Table: Table);

/// Types that can be used to index a `toml::ValueKind`
///
/// Currently this is implemented for `usize` to index arrays and `str` to index
/// tables.
///
/// This trait is sealed and not intended for implementation outside of the
/// `toml` crate.
pub trait Index: Sealed {
    #[doc(hidden)]
    fn index<'a>(&self, val: &'a ValueKind) -> Option<&'a SpannedValue>;
    #[doc(hidden)]
    fn index_mut<'a>(&self, val: &'a mut ValueKind) -> Option<&'a mut SpannedValue>;
}

/// An implementation detail that should not be implemented, this will change in
/// the future and break code otherwise.
#[doc(hidden)]
pub trait Sealed {}
impl Sealed for usize {}
impl Sealed for str {}
impl Sealed for String {}
impl<'a, T: Sealed + ?Sized> Sealed for &'a T {}

impl Index for usize {
    fn index<'a>(&self, val: &'a ValueKind) -> Option<&'a SpannedValue> {
        match *val {
            ValueKind::Array(ref a) => a.get(*self),
            _ => None,
        }
    }

    fn index_mut<'a>(&self, val: &'a mut ValueKind) -> Option<&'a mut SpannedValue> {
        match *val {
            ValueKind::Array(ref mut a) => a.get_mut(*self),
            _ => None,
        }
    }
}

impl Index for str {
    fn index<'a>(&self, val: &'a ValueKind) -> Option<&'a SpannedValue> {
        match *val {
            ValueKind::Table(ref a) => a.get(self),
            _ => None,
        }
    }

    fn index_mut<'a>(&self, val: &'a mut ValueKind) -> Option<&'a mut SpannedValue> {
        match *val {
            ValueKind::Table(ref mut a) => a.get_mut(self),
            _ => None,
        }
    }
}

impl Index for String {
    fn index<'a>(&self, val: &'a ValueKind) -> Option<&'a SpannedValue> {
        self[..].index(val)
    }

    fn index_mut<'a>(&self, val: &'a mut ValueKind) -> Option<&'a mut SpannedValue> {
        self[..].index_mut(val)
    }
}

impl<'s, T: ?Sized> Index for &'s T
where
    T: Index,
{
    fn index<'a>(&self, val: &'a ValueKind) -> Option<&'a SpannedValue> {
        (**self).index(val)
    }

    fn index_mut<'a>(&self, val: &'a mut ValueKind) -> Option<&'a mut SpannedValue> {
        (**self).index_mut(val)
    }
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::ser::to_string(self)
            .expect("Unable to represent value as string")
            .fmt(f)
    }
}

impl FromStr for ValueKind {
    type Err = crate::de::Error;
    fn from_str(s: &str) -> Result<ValueKind, Self::Err> {
        crate::from_str(s)
    }
}

impl ser::Serialize for ValueKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeMap;

        match *self {
            ValueKind::String(ref s) => serializer.serialize_str(s),
            ValueKind::Integer(i) => serializer.serialize_i64(i),
            ValueKind::Float(f) => serializer.serialize_f64(f),
            ValueKind::Boolean(b) => serializer.serialize_bool(b),
            ValueKind::Datetime(ref s) => s.serialize(serializer),
            ValueKind::Array(ref a) => a.serialize(serializer),
            ValueKind::Table(ref t) => {
                let mut map = serializer.serialize_map(Some(t.len()))?;
                // Be sure to visit non-tables first (and also non
                // array-of-tables) as all keys must be emitted first.
                for (k, v) in t {
                    if !v.get_ref().is_table() && !v.get_ref().is_array()
                        || (v
                            .get_ref()
                            .as_array()
                            .map(|a| !a.iter().any(|v| v.get_ref().is_table()))
                            .unwrap_or(false))
                    {
                        map.serialize_entry(k, v)?;
                    }
                }
                for (k, v) in t {
                    if v.get_ref()
                        .as_array()
                        .map(|a| a.iter().any(|v| v.get_ref().is_table()))
                        .unwrap_or(false)
                    {
                        map.serialize_entry(k, v)?;
                    }
                }
                for (k, v) in t {
                    if v.get_ref().is_table() {
                        map.serialize_entry(k, v)?;
                    }
                }
                map.end()
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for ValueKind {
    fn deserialize<D>(deserializer: D) -> Result<ValueKind, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ValueKindVisitor;

        impl<'de> de::Visitor<'de> for ValueKindVisitor {
            type Value = ValueKind;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any valid TOML value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<ValueKind, E> {
                Ok(ValueKind::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<ValueKind, E> {
                Ok(ValueKind::Integer(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<ValueKind, E> {
                if value <= i64::max_value() as u64 {
                    Ok(ValueKind::Integer(value as i64))
                } else {
                    Err(de::Error::custom("u64 value was too large"))
                }
            }

            fn visit_u32<E>(self, value: u32) -> Result<ValueKind, E> {
                Ok(ValueKind::Integer(value.into()))
            }

            fn visit_i32<E>(self, value: i32) -> Result<ValueKind, E> {
                Ok(ValueKind::Integer(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<ValueKind, E> {
                Ok(ValueKind::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<ValueKind, E> {
                Ok(ValueKind::String(value.into()))
            }

            fn visit_string<E>(self, value: String) -> Result<ValueKind, E> {
                Ok(ValueKind::String(value))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<ValueKind, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<ValueKind, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }
                Ok(ValueKind::Array(vec))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<ValueKind, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let key = visitor.next_key_seed(DatetimeOrTable)?;
                let key = match key {
                    Some(Some(key)) => key,
                    Some(None) => {
                        let date: DatetimeFromString = visitor.next_value()?;
                        return Ok(ValueKind::Datetime(date.value));
                    }
                    None => return Ok(ValueKind::Table(Map::new())),
                };
                let mut map = Map::new();
                map.insert(key, visitor.next_value()?);
                while let Some(key) = visitor.next_key()? {
                    if map.contains_key(&key) {
                        let key: Spanned<String> = key;
                        let msg = format!("duplicate key: `{}`", key.get_ref());
                        return Err(de::Error::custom(msg));
                    }
                    map.insert(key, visitor.next_value()?);
                }
                Ok(ValueKind::Table(map))
            }
        }

        deserializer.deserialize_any(ValueKindVisitor)
    }
}

struct DatetimeOrTable;

impl<'de> de::DeserializeSeed<'de> for DatetimeOrTable {
    type Value = Option<Spanned<String>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        static FIELDS: [&str; 3] = [spanned::START, spanned::END, spanned::VALUE];
        deserializer.deserialize_struct(spanned::NAME, &FIELDS, self)
    }
}

impl<'de> de::Visitor<'de> for DatetimeOrTable {
    type Value = Option<Spanned<String>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string key")
    }

    fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let spanned_visitor = spanned::SpannedVisitor(::std::marker::PhantomData);
        let key = spanned_visitor.visit_map(visitor)?;
        Ok(Some(key))
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        assert_eq!(s, datetime::FIELD);
        Ok(None)
    }

    fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        assert_eq!(s, datetime::FIELD);
        Ok(None)
    }
}
