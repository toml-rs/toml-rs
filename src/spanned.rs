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
                if visitor.next_key()? != Some(START) {
                    return Err(de::Error::custom("spanned start key not found"))
                }

                let start: usize = visitor.next_value()?;

                if visitor.next_key()? != Some(END) {
                    return Err(de::Error::custom("spanned end key not found"))
                }

                let end: usize = visitor.next_value()?;

                if visitor.next_key()? != Some(VALUE) {
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
