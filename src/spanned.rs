//! ```
//! #[macro_use]
//! extern crate serde_derive;
//!
//! extern crate toml;
//! use toml::spanned::Spanned;
//!
//! #[derive(Deserialize)]
//! struct Value {
//!     s: Spanned<String>,
//! }
//!
//! fn main() {
//!     let t = "s = \"value\"\n";
//!
//!     let u: Value = toml::from_str(t).unwrap();
//!
//!     assert_eq!(u.s.start, 4);
//!     assert_eq!(u.s.end, 11);
//! }
//! ```

use serde::{de, ser};
use std::fmt;

#[doc(hidden)]
pub const NAME: &'static str = "$__toml_private_Spanned";
#[doc(hidden)]
pub const START: &'static str = "$__toml_private_start";
#[doc(hidden)]
pub const END: &'static str = "$__toml_private_end";
#[doc(hidden)]
pub const VALUE: &'static str = "$__toml_private_value";

macro_rules! key_deserialize {
    ($ident:ident, $field:expr, $name:expr) => {
        struct $ident;

        impl<'de> de::Deserialize<'de> for $ident {
            fn deserialize<D>(deserializer: D) -> Result<$ident, D::Error>
                where D: de::Deserializer<'de>
            {
                struct FieldVisitor;

                impl<'de> de::Visitor<'de> for FieldVisitor {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a valid spanned field")
                    }

                    fn visit_str<E>(self, s: &str) -> Result<(), E>
                        where E: de::Error
                    {
                        if s == $field {
                            Ok(())
                        } else {
                            Err(de::Error::custom(
                                concat!("expected spanned field `", $name, "`")))
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)?;
                Ok($ident)
            }
        }
    }
}


/// A spanned value, indicating the range at which it is defined in the source.
#[derive(Debug)]
pub struct Spanned<T> {
    /// The start range.
    pub start: usize,
    /// The end range (exclusive).
    pub end: usize,
    /// The spanned value.
    pub value: T,
}

impl<'de, T> de::Deserialize<'de> for Spanned<T>
    where T: de::Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Spanned<T>, D::Error>
        where D: de::Deserializer<'de>
    {
        struct SpannedVisitor<T>(::std::marker::PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for SpannedVisitor<T>
            where T: de::Deserialize<'de>
        {
            type Value = Spanned<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a TOML spanned")
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Spanned<T>, V::Error>
                where V: de::MapAccess<'de>
            {
                let start = visitor.next_key::<StartKey>()?;

                if start.is_none() {
                    return Err(de::Error::custom("spanned start key not found"))
                }

                let start: usize = visitor.next_value()?;

                let end = visitor.next_key::<EndKey>()?;

                if end.is_none() {
                    return Err(de::Error::custom("spanned end key not found"))
                }

                let end: usize = visitor.next_value()?;

                let value = visitor.next_key::<ValueKey>()?;

                if value.is_none() {
                    return Err(de::Error::custom("spanned value key not found"))
                }

                let value: T = visitor.next_value()?;

                Ok(Spanned {
                    start: start,
                    end: end,
                    value: value
                })
            }
        }

        key_deserialize!(StartKey, START, "start");
        key_deserialize!(EndKey, END, "end");
        key_deserialize!(ValueKey, VALUE, "value");

        let visitor = SpannedVisitor(::std::marker::PhantomData);

        static FIELDS: [&'static str; 3] = [START, END, VALUE];
        deserializer.deserialize_struct(NAME, &FIELDS, visitor)
    }
}

impl<T: ser::Serialize> ser::Serialize for Spanned<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.value.serialize(serializer)
    }
}
